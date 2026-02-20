use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct StreamingSink {
    format: AudioFormat,
    tx: UnboundedSender<Vec<u8>>,
}

impl StreamingSink {
    pub fn new(format: AudioFormat, tx: UnboundedSender<Vec<u8>>) -> Self {
        Self { format, tx }
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
        _ = self.tx.send(bytes);
        Ok(())
    }
}
