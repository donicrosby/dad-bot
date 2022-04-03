use getset::{Getters, Setters};
use rand::{RngCore};

#[derive(Debug, Clone)]
pub enum EnabledChance {
    Off,
    Always,
    In(i64)
}

#[derive(Debug, Clone, Getters, Setters)]
pub struct RngManager<R: RngCore + Send> {
    #[getset(get, set = "pub")]
    dadded_chance: EnabledChance,
    #[getset(get, set = "pub")]
    love_me_chance: EnabledChance,
    #[getset(get, set = "pub")]
    rng: R,
}

impl<R> RngManager<R>
where R: RngCore + Send {
    pub fn new(dadded_chance: Option<i64>, love_me_chance: Option<i64>, rng: R) -> Self {
        let dadded_chance = match dadded_chance {
            None => EnabledChance::Always,
            Some(c) => {
                if c > 1 {
                    EnabledChance::In(c)
                } else {
                    EnabledChance::Always
                }
            }
        };
        let love_me_chance = match love_me_chance {
            None => EnabledChance::Off,
            Some(c) => {
                match c {
                    0 => EnabledChance::Off,
                    1 => EnabledChance::Always,
                    c => EnabledChance::In(c),
                }
            }
        };

        Self {
            dadded_chance,
            love_me_chance,
            rng
        }
    }

    pub fn should_dad(&mut self) -> bool {
        match self.dadded_chance {
            EnabledChance::In(c) => self.get_n_in_chance(c),
            _ => true,
        }
    }

    pub fn should_love_you(&mut self) -> bool {

        match self.love_me_chance {
            EnabledChance::Off => false,
            EnabledChance::Always => true,
            EnabledChance::In(c) => self.get_n_in_chance(c)
        }
    }

    fn get_n_in_chance(&mut self, chance: i64) -> bool {
        let roll = self.rng.next_u64();
        let c = chance as u64;
        (roll % c) == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::mock::StepRng;
    use crate::errors::Error;

    #[tokio::test]
    async fn test_should_dad() -> Result<(), Error> {
        let mock_rng = StepRng::new(10, 1);
        let dad_chance = Some(10);
        let love_me_chance = None;
        let mut mgr = RngManager::new(dad_chance, love_me_chance, mock_rng);
        let should_dad = mgr.should_dad();
        assert_eq!(should_dad, true);
        let should_dad = mgr.should_dad();
        assert_eq!(should_dad, false);
        Ok(())
    }

    #[tokio::test]
    async fn test_always_dad_some() -> Result<(), Error> {
        let mock_rng = StepRng::new(10, 1);
        let dad_chance = Some(1);
        let love_me_chance = None;
        let mut mgr = RngManager::new(dad_chance, love_me_chance, mock_rng);
        let should_dad = mgr.should_dad();
        assert_eq!(should_dad, true);
        let should_dad = mgr.should_dad();
        assert_eq!(should_dad, true);
        Ok(())
    }

    #[tokio::test]
    async fn test_always_dad_none() -> Result<(), Error> {
        let mock_rng = StepRng::new(10, 1);
        let dad_chance = None;
        let love_me_chance = None;
        let mut mgr = RngManager::new(dad_chance, love_me_chance, mock_rng);
        let should_dad = mgr.should_dad();
        assert_eq!(should_dad, true);
        let should_dad = mgr.should_dad();
        assert_eq!(should_dad, true);
        Ok(())
    }

    #[tokio::test]
    async fn test_should_love_you() -> Result<(), Error> {
        let mock_rng = StepRng::new(10, 1);
        let dad_chance = None;
        let love_me_chance = Some(10);
        let mut mgr = RngManager::new(dad_chance, love_me_chance, mock_rng);
        let should_love_you = mgr.should_love_you();
        assert_eq!(should_love_you, true);
        let should_love_you = mgr.should_love_you();
        assert_eq!(should_love_you, false);
        Ok(())
    }

    #[tokio::test]
    async fn test_should_love_you_always() -> Result<(), Error> {
        let mock_rng = StepRng::new(10, 1);
        let dad_chance = None;
        let love_me_chance = Some(1);
        let mut mgr = RngManager::new(dad_chance, love_me_chance, mock_rng);
        let should_love_you = mgr.should_love_you();
        assert_eq!(should_love_you, true);
        let should_love_you = mgr.should_love_you();
        assert_eq!(should_love_you, true);
        Ok(())
    }

    #[tokio::test]
    async fn test_should_love_you_never() -> Result<(), Error> {
        let mock_rng = StepRng::new(10, 1);
        let dad_chance = None;
        let love_me_chance = None;
        let mut mgr = RngManager::new(dad_chance, love_me_chance, mock_rng);
        let should_love_you = mgr.should_love_you();
        assert_eq!(should_love_you, false);
        let should_love_you = mgr.should_love_you();
        assert_eq!(should_love_you, false);
        Ok(())
    }
}
