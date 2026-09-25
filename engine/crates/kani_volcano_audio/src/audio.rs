use anyhow::{anyhow, Context, Result};
use kira::{
    sound::static_sound::StaticSoundData,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AudioHandle(pub usize);

pub struct DecodedAudio {
    pub data: StaticSoundData,
}

impl DecodedAudio {
    pub fn from_file(path: &str) -> Result<Self> {
        let data = StaticSoundData::from_file(path)
            .with_context(|| format!("failed to decode audio: {path}"))?;

        Ok(Self { data })
    }
}