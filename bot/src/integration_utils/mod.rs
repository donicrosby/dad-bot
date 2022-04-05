#[cfg(test)]
mod utils {
    use crate::errors::Error;
    use db::migration::*;
    use db::sea_orm::*;
    use rand::rngs::mock::StepRng;
    use rand::{RngCore, SeedableRng};

    pub async fn create_inmemory_db() -> Result<DbConn, Error> {
        let db = Database::connect("sqlite::memory:").await?;
        Migrator::up(&db, None).await?;
        Ok(db)
    }

    const N: usize = 8;
    pub struct SeedableStepSeed(pub [u8; N]);

    pub struct SeedableStepRng {
        rng: StepRng,
    }

    impl Default for SeedableStepSeed {
        fn default() -> SeedableStepSeed {
            SeedableStepSeed([0; N])
        }
    }

    impl AsMut<[u8]> for SeedableStepSeed {
        fn as_mut(&mut self) -> &mut [u8] {
            &mut self.0
        }
    }

    impl SeedableStepRng {
        pub fn new(inital: u64, increment: u64) -> Self {
            Self {
                rng: StepRng::new(inital, increment),
            }
        }
    }

    impl RngCore for SeedableStepRng {
        fn next_u64(&mut self) -> u64 {
            self.rng.next_u64()
        }

        fn next_u32(&mut self) -> u32 {
            self.next_u64() as u32
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            self.rng.fill_bytes(dest);
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
            self.rng.try_fill_bytes(dest)
        }
    }

    impl SeedableRng for SeedableStepRng {
        type Seed = SeedableStepSeed;

        fn from_seed(seed: SeedableStepSeed) -> SeedableStepRng {
            let init = u64::from_ne_bytes(seed.0);
            Self {
                rng: StepRng::new(init, 1),
            }
        }
    }
}

pub use self::utils::create_inmemory_db;
pub use self::utils::SeedableStepRng;
