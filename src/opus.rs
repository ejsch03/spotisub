use std::io::Cursor;

use audiopus::{Application, Channels, SampleRate, coder::Encoder};
use ogg::writing::PacketWriteEndInfo;
use rubato::{
    Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction, audioadapter::Adapter,
};

// opus frame size at 48kHz — 20ms per frame, stereo interleaved
const OPUS_FRAME_SIZE: usize = 960;
const OPUS_FRAME_SAMPLES: usize = OPUS_FRAME_SIZE * 2;

// number of samples to trim from the start to compensate for encoder algorithmic delay
const OPUS_PRE_SKIP: u16 = 312;

// resampler input chunk size — larger = more latency, smaller = more cpu
const CHUNK_SIZE: usize = 960;

/// drives pcm data from librespot (44100hz s16 stereo) through resampling and
/// opus encoding into a valid ogg/opus bytestream.
pub struct AudioPipeline {
    pub encoder: OggOpusStreamer,
    resampler: Async<f32>,
    buffer_left: Vec<f32>,
    buffer_right: Vec<f32>,
    resample_left: Vec<f32>,
    resample_right: Vec<f32>,
}

impl AudioPipeline {
    pub fn new() -> Self {
        let params = SincInterpolationParameters {
            sinc_len: 64,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 128,
            window: WindowFunction::BlackmanHarris2,
        };

        // resampler converts 44100hz (librespot output) → 48000hz (opus requirement)
        let resampler = Async::<f32>::new_sinc(
            48000.0 / 44100.0,
            2.0,
            &params,
            CHUNK_SIZE,
            2,
            FixedAsync::Input,
        )
        .expect("failed to create resampler");

        Self {
            encoder: OggOpusStreamer::new(),
            resampler,
            buffer_left: Vec::with_capacity(CHUNK_SIZE * 2),
            buffer_right: Vec::with_capacity(CHUNK_SIZE * 2),
            resample_left: vec![0.0; CHUNK_SIZE],
            resample_right: vec![0.0; CHUNK_SIZE],
        }
    }

    /// resamples one chunk of left/right f32 samples and returns stereo interleaved i16.
    fn resample_and_interleave(&mut self) -> Vec<i16> {
        let input = vec![self.resample_left.clone(), self.resample_right.clone()];
        let input_adapter =
            audioadapter_buffers::direct::SequentialSliceOfVecs::new(&input, 2, CHUNK_SIZE)
                .expect("failed to create input adapter");

        let resampled = self
            .resampler
            .process(&input_adapter, 0, None)
            .expect("resampling failed");

        let frames = resampled.frames();
        let mut interleaved = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            let l = resampled.read_sample(0, i).unwrap_or(0.0);
            let r = resampled.read_sample(1, i).unwrap_or(0.0);
            // clamp before cast to avoid undefined behaviour on out-of-range f32 values
            interleaved.push((l * 32767.0).clamp(-32768.0, 32767.0) as i16);
            interleaved.push((r * 32767.0).clamp(-32768.0, 32767.0) as i16);
        }
        interleaved
    }

    /// accepts raw s16 stereo pcm, buffers internally, and returns any newly encoded ogg pages.
    pub fn process(&mut self, pcm: &[i16]) -> Vec<u8> {
        // deinterleave and normalise to f32 for the resampler
        for pair in pcm.chunks_exact(2) {
            self.buffer_left.push(pair[0] as f32 * (1.0 / 32768.0));
            self.buffer_right.push(pair[1] as f32 * (1.0 / 32768.0));
        }

        let mut all_ogg = Vec::new();

        while self.buffer_left.len() >= CHUNK_SIZE {
            self.resample_left
                .copy_from_slice(&self.buffer_left[..CHUNK_SIZE]);
            self.resample_right
                .copy_from_slice(&self.buffer_right[..CHUNK_SIZE]);

            self.buffer_left.drain(..CHUNK_SIZE);
            self.buffer_right.drain(..CHUNK_SIZE);

            let interleaved = self.resample_and_interleave();
            all_ogg.extend(self.encoder.encode_chunk(&interleaved));
        }

        all_ogg
    }

    /// flushes any remaining buffered samples and finalises the ogg stream.
    pub fn flush(&mut self) -> Vec<u8> {
        let mut out = Vec::new();

        if !self.buffer_left.is_empty() {
            // pad the final chunk to a full chunk size with silence
            self.buffer_left.resize(CHUNK_SIZE, 0.0);
            self.buffer_right.resize(CHUNK_SIZE, 0.0);

            self.resample_left
                .copy_from_slice(&self.buffer_left[..CHUNK_SIZE]);
            self.resample_right
                .copy_from_slice(&self.buffer_right[..CHUNK_SIZE]);
            self.buffer_left.clear();
            self.buffer_right.clear();

            let interleaved = self.resample_and_interleave();
            out.extend(self.encoder.encode_chunk(&interleaved));
        }

        out.extend(self.encoder.flush());
        out
    }
}

/// wraps an opus encoder and ogg muxer, producing a valid streaming ogg/opus bytestream.
pub struct OggOpusStreamer {
    encoder: Encoder,
    ogg_writer: ogg::writing::PacketWriter<'static, Cursor<Vec<u8>>>,
    // granule position counts opus samples at 48kHz, used for seeking and sync
    granule_pos: u64,
    // random serial number identifies this logical ogg bitstream
    serial: u32,
    // internal sample buffer to accumulate until a full opus frame is available
    buffer: Vec<i16>,
    encode_buf: Vec<u8>,
}

impl OggOpusStreamer {
    pub fn new() -> Self {
        let encoder = Encoder::new(SampleRate::Hz48000, Channels::Stereo, Application::Audio)
            .expect("failed to create opus encoder");

        Self {
            encoder,
            ogg_writer: ogg::writing::PacketWriter::new(Cursor::new(Vec::new())),
            granule_pos: 0,
            serial: rand::random::<u32>(),
            buffer: Vec::new(),
            // 4000 bytes is well above the maximum opus packet size
            encode_buf: vec![0u8; 4000],
        }
    }

    /// builds and returns the ogg/opus identification and comment headers.
    pub fn header_bytes(&mut self) -> Vec<u8> {
        // OpusHead identification header — defined in rfc7845 §5.1
        let mut head = Vec::new();
        head.extend_from_slice(b"OpusHead");
        head.push(1); // version
        head.push(2); // channel count
        head.extend_from_slice(&OPUS_PRE_SKIP.to_le_bytes());
        head.extend_from_slice(&48000u32.to_le_bytes()); // input sample rate (informational)
        head.extend_from_slice(&0i16.to_le_bytes()); // output gain
        head.push(0); // channel mapping family (stereo)

        self.ogg_writer
            .write_packet(head, self.serial, PacketWriteEndInfo::EndPage, 0)
            .expect("failed to write OpusHead");

        // OpusTags comment header — required by spec even if empty
        let mut tags = Vec::new();
        tags.extend_from_slice(b"OpusTags");
        let vendor = env!("CARGO_PKG_NAME").as_bytes();
        tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        tags.extend_from_slice(vendor);
        tags.extend_from_slice(&0u32.to_le_bytes()); // zero user comments

        self.ogg_writer
            .write_packet(tags, self.serial, PacketWriteEndInfo::EndPage, 0)
            .expect("failed to write OpusTags");

        self.take_ogg_bytes()
    }

    /// encodes interleaved s16 stereo pcm into ogg pages and returns any completed pages.
    pub fn encode_chunk(&mut self, pcm: &[i16]) -> Vec<u8> {
        self.buffer.extend_from_slice(pcm);

        while self.buffer.len() >= OPUS_FRAME_SAMPLES {
            let frame: Vec<i16> = self.buffer.drain(..OPUS_FRAME_SAMPLES).collect();
            let len = self
                .encoder
                .encode(&frame, &mut self.encode_buf)
                .expect("opus encode failed");

            self.granule_pos += OPUS_FRAME_SIZE as u64;

            self.ogg_writer
                .write_packet(
                    self.encode_buf[..len].to_vec(),
                    self.serial,
                    PacketWriteEndInfo::NormalPacket,
                    self.granule_pos,
                )
                .expect("failed to write ogg packet");
        }

        self.take_ogg_bytes()
    }

    /// encodes any remaining buffered samples and writes the ogg end-of-stream marker.
    pub fn flush(&mut self) -> Vec<u8> {
        if !self.buffer.is_empty() {
            // pad the final frame with silence so the encoder receives a full frame
            self.buffer.resize(OPUS_FRAME_SAMPLES, 0);
            let frame = std::mem::take(&mut self.buffer);
            let len = self
                .encoder
                .encode(&frame, &mut self.encode_buf)
                .expect("opus encode failed during flush");

            self.granule_pos += OPUS_FRAME_SIZE as u64;

            self.ogg_writer
                .write_packet(
                    self.encode_buf[..len].to_vec(),
                    self.serial,
                    PacketWriteEndInfo::EndStream,
                    self.granule_pos,
                )
                .expect("failed to write final ogg packet");
        } else {
            // stream was frame-aligned — still need EndStream for a valid bitstream
            self.ogg_writer
                .write_packet(
                    vec![],
                    self.serial,
                    PacketWriteEndInfo::EndStream,
                    self.granule_pos,
                )
                .expect("failed to write ogg end-of-stream marker");
        }
        self.take_ogg_bytes()
    }

    /// extracts all bytes written to the internal cursor and resets it for the next write.
    fn take_ogg_bytes(&mut self) -> Vec<u8> {
        let cursor = self.ogg_writer.inner_mut();
        let bytes = cursor.get_ref().clone();
        cursor.get_mut().clear();
        cursor.set_position(0);
        bytes
    }
}
