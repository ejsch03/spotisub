use std::io::Cursor;

use audiopus::{Application, Channels, SampleRate, coder::Encoder};
use ogg::writing::PacketWriteEndInfo;
use rubato::audioadapter::Adapter;
use rubato::{
    Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

pub struct AudioPipeline {
    pub encoder: OggOpusStreamer,
    resampler: Async<f32>,
    buffer_left: Vec<f32>,
    buffer_right: Vec<f32>,
    resample_left: Vec<f32>,
    resample_right: Vec<f32>,
    chunk_size: usize,
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

        let chunk_size = 4096;
        let resampler = Async::<f32>::new_sinc(
            48000.0 / 44100.0,
            2.0,
            &params,
            chunk_size,
            2,
            FixedAsync::Input,
        )
        .unwrap();

        Self {
            encoder: OggOpusStreamer::new(),
            resampler,
            buffer_left: Vec::with_capacity(chunk_size * 2),
            buffer_right: Vec::with_capacity(chunk_size * 2),
            resample_left: vec![0.0; chunk_size],
            resample_right: vec![0.0; chunk_size],
            chunk_size,
        }
    }

    fn resample_and_interleave(&mut self) -> Vec<i16> {
        let input = vec![self.resample_left.clone(), self.resample_right.clone()];
        let input_adapter =
            audioadapter_buffers::direct::SequentialSliceOfVecs::new(&input, 2, self.chunk_size)
                .unwrap();

        let resampled = self.resampler.process(&input_adapter, 0, None).unwrap();

        let frames = resampled.frames();
        let mut interleaved = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            let l: f32 = resampled.read_sample(0, i).unwrap_or(0.0);
            let r: f32 = resampled.read_sample(1, i).unwrap_or(0.0);
            interleaved.push((l * 32767.0).clamp(-32768.0, 32767.0) as i16);
            interleaved.push((r * 32767.0).clamp(-32768.0, 32767.0) as i16);
        }
        interleaved
    }

    pub fn process(&mut self, pcm: &[i16]) -> Vec<u8> {
        for pair in pcm.chunks_exact(2) {
            self.buffer_left.push(pair[0] as f32 * (1.0 / 32768.0));
            self.buffer_right.push(pair[1] as f32 * (1.0 / 32768.0));
        }

        let mut all_ogg = Vec::new();

        while self.buffer_left.len() >= self.chunk_size {
            self.resample_left
                .copy_from_slice(&self.buffer_left[..self.chunk_size]);
            self.resample_right
                .copy_from_slice(&self.buffer_right[..self.chunk_size]);

            self.buffer_left.drain(..self.chunk_size);
            self.buffer_right.drain(..self.chunk_size);

            let interleaved = self.resample_and_interleave();
            all_ogg.extend(self.encoder.encode_chunk(&interleaved));
        }

        all_ogg
    }

    pub fn flush(&mut self) -> Vec<u8> {
        let mut out = Vec::new();

        if !self.buffer_left.is_empty() {
            self.buffer_left.resize(self.chunk_size, 0.0);
            self.buffer_right.resize(self.chunk_size, 0.0);

            self.resample_left
                .copy_from_slice(&self.buffer_left[..self.chunk_size]);
            self.resample_right
                .copy_from_slice(&self.buffer_right[..self.chunk_size]);
            self.buffer_left.clear();
            self.buffer_right.clear();

            let interleaved = self.resample_and_interleave();
            out.extend(self.encoder.encode_chunk(&interleaved));
        }

        out.extend(self.encoder.flush());
        out
    }
}

pub struct OggOpusStreamer {
    encoder: Encoder,
    ogg_writer: ogg::writing::PacketWriter<'static, Cursor<Vec<u8>>>,
    granule_pos: u64,
    serial: u32,
    buffer: Vec<i16>,
    encode_buf: Vec<u8>,
}

impl OggOpusStreamer {
    pub fn new() -> Self {
        let encoder =
            Encoder::new(SampleRate::Hz48000, Channels::Stereo, Application::Audio).unwrap();

        Self {
            encoder,
            ogg_writer: ogg::writing::PacketWriter::new(Cursor::new(Vec::new())),
            granule_pos: 0,
            serial: rand::random::<u32>(),
            buffer: Vec::new(),
            encode_buf: vec![0u8; 4000],
        }
    }

    pub fn header_bytes(&mut self) -> Vec<u8> {
        let mut head = vec![];
        head.extend_from_slice(b"OpusHead");
        head.push(1);
        head.push(2);
        head.extend_from_slice(&0u16.to_le_bytes());
        head.extend_from_slice(&48000u32.to_le_bytes());
        head.extend_from_slice(&0i16.to_le_bytes());
        head.push(0);

        self.ogg_writer
            .write_packet(head, self.serial, PacketWriteEndInfo::EndPage, 0)
            .unwrap();

        let mut tags = vec![];
        tags.extend_from_slice(b"OpusTags");
        let vendor = env!("CARGO_PKG_NAME").as_bytes();
        tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        tags.extend_from_slice(vendor);
        tags.extend_from_slice(&0u32.to_le_bytes());

        self.ogg_writer
            .write_packet(tags, self.serial, PacketWriteEndInfo::EndPage, 0)
            .unwrap();

        self.take_ogg_bytes()
    }

    pub fn encode_chunk(&mut self, pcm: &[i16]) -> Vec<u8> {
        self.buffer.extend_from_slice(pcm);

        let frame_samples = 960 * 2;

        while self.buffer.len() >= frame_samples {
            let frame: Vec<i16> = self.buffer.drain(..frame_samples).collect();

            let len = self.encoder.encode(&frame, &mut self.encode_buf).unwrap();
            self.granule_pos += 960;

            self.ogg_writer
                .write_packet(
                    self.encode_buf[..len].to_vec(),
                    self.serial,
                    PacketWriteEndInfo::NormalPacket,
                    self.granule_pos,
                )
                .unwrap();
        }

        self.take_ogg_bytes()
    }

    pub fn flush(&mut self) -> Vec<u8> {
        if self.buffer.is_empty() {
            return Vec::new();
        }

        self.buffer.resize(960 * 2, 0);
        let frame = std::mem::take(&mut self.buffer);

        let len = self.encoder.encode(&frame, &mut self.encode_buf).unwrap();
        self.granule_pos += 960;

        self.ogg_writer
            .write_packet(
                self.encode_buf[..len].to_vec(),
                self.serial,
                PacketWriteEndInfo::EndStream,
                self.granule_pos,
            )
            .unwrap();

        self.take_ogg_bytes()
    }

    fn take_ogg_bytes(&mut self) -> Vec<u8> {
        let cursor = self.ogg_writer.inner_mut();
        let bytes = cursor.get_ref().clone();
        cursor.set_position(0);
        cursor.get_mut().clear();
        bytes
    }
}
