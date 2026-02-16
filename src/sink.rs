use librespot::playback::{
    audio_backend::{Sink, SinkResult},
    config::AudioFormat,
    convert::Converter,
    decoder::AudioPacket,
};
use parking_lot::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use zerocopy::IntoBytes;

use crate::*;

#[derive(Clone, Debug)]
pub struct StreamingSink {
    tx: Arc<Mutex<Option<UnboundedSender<Vec<u8>>>>>,
    format: AudioFormat,
}

impl StreamingSink {
    pub fn new(format: AudioFormat) -> Self {
        Self {
            tx: Default::default(),
            format,
        }
    }

    pub fn set_sender(&self, tx: UnboundedSender<Vec<u8>>) {
        *self.tx.lock() = Some(tx)
    }
}

impl Sink for StreamingSink {
    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        let bytes = match packet {
            AudioPacket::Samples(samples) => match self.format {
                AudioFormat::F64 => samples.as_bytes().to_vec(),
                AudioFormat::F32 => converter.f64_to_f32(&samples).as_bytes().to_vec(),
                AudioFormat::S32 => converter.f64_to_s32(&samples).as_bytes().to_vec(),
                AudioFormat::S24 => converter.f64_to_s24(&samples).as_bytes().to_vec(),
                AudioFormat::S24_3 => converter.f64_to_s24_3(&samples).as_bytes().to_vec(),
                AudioFormat::S16 => converter.f64_to_s16(&samples).as_bytes().to_vec(),
            },
            AudioPacket::Raw(bytes) => bytes,
        };

        // Receiver dropped = client disconnected, silently stop
        if let Some(tx) = &*self.tx.lock() {
            _ = tx.send(bytes);
        } else {
            log::warn!("no avaiable UnboundedSender<Vec<u8>>!");
        }
        Ok(())
    }
}
