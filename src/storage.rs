use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embedded_storage::{ReadStorage, Storage};
use esp_storage::FlashStorage;
use serde::{Deserialize, Serialize};

use crate::data::{Calibration, Millimeters};

static CONFIGURATION: Mutex<CriticalSectionRawMutex, Data> = Mutex::new(Data::const_default());

const MAGIC_BYTES: [u8; 4] = [123, 52, 61, 53];
const FLASH_ADDR: u32 = 0x9000;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct InnerData {
    position_1: Option<Millimeters>,
    position_2: Option<Millimeters>,
    calibration: Calibration,
}

impl InnerData {
    const fn const_default() -> Self {
        Self {
            position_1: None,
            position_2: None,
            calibration: Calibration::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Data {
    magic_identifier: [u8; 4],
    inner: InnerData,
}

impl Data {
    pub const fn const_default() -> Self {
        Self {
            magic_identifier: [0; 4],
            inner: InnerData::const_default(),
        }
    }

    pub fn get(&mut self) -> &InnerData {
        self.init_inner();
        &self.inner
    }

    fn init_inner(&mut self) {
        if self.magic_identifier == MAGIC_BYTES {
            return;
        }

        *self = Self::load().unwrap_or_else(Self::const_default);
        self.magic_identifier = MAGIC_BYTES;
    }

    fn load() -> Option<Self> {
        let mut bytes = [0u8; core::mem::size_of::<Data>()];
        FlashStorage::new()
            .read(FLASH_ADDR, &mut bytes)
            .inspect_err(|e| log::error!("failed to read flash storage: {e:?}"))
            .ok()?;

        postcard::from_bytes(&bytes)
            .inspect_err(|e| {
                log::error!(
                    "failed to load configuration: {e}\nThis is normal during first-time use."
                )
            })
            .ok()
    }

    fn store(&self) {
        const DATA_SIZE: usize = core::mem::size_of::<Data>();
        let Ok(bytes) = postcard::to_vec::<_, DATA_SIZE>(self)
            .inspect_err(|e| log::error!("failed to serialize configuration: {e}"))
            else {return;};

        let _ = FlashStorage::new()
            .write(FLASH_ADDR, &bytes)
            .inspect_err(|e| log::error!("failed to write flash storage: {e:?}"));
    }

    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(&mut InnerData),
    {
        self.init_inner();
        f(&mut self.inner);
    }
}
