use std::collections::HashMap;
use crate::types::PubKey;

pub struct ReputationBook {
    scores: HashMap<PubKey, f64>,
    reward_step: f64,
    punish_step: f64,
    floor: f64,
    ceiling: f64,
    neutral: f64,
    admit_threshold: f64,
}

impl ReputationBook {
    pub fn new() -> Self {
        Self {
            scores: HashMap::new(),
            reward_step: 0.1,
            punish_step: 0.2,
            floor: 0.0,
            ceiling: 1.0,
            neutral: 0.5,
            admit_threshold: 0.30,
        }
    }

    pub fn get(&self, who: &PubKey) -> f64 {
        *self.scores.get(who).unwrap_or(&self.neutral)
    }

    pub fn reward(&mut self, who: &PubKey) {
        let e = self.scores.entry(who.clone()).or_insert(self.neutral);
        *e = (*e + self.reward_step).min(self.ceiling);
    }

    pub fn punish(&mut self, who: &PubKey) {
        let e = self.scores.entry(who.clone()).or_insert(self.neutral);
        *e = (*e - self.punish_step).max(self.floor);
    }

    pub fn decay(&mut self) {
        let neutral = self.neutral;
        for (_, score) in self.scores.iter_mut() {
            if *score < neutral {
                let delta = 0.1 * (neutral - *score);
                *score += delta;
            }
        }
    }

    pub fn admit_threshold(&self) -> f64 {
        self.admit_threshold
    }
}
