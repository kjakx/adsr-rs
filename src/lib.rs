#[derive(Copy, Clone, PartialEq)]
pub enum ADSREvent {
    NoteOn,
    NoteOff,
}

#[derive(Copy, Clone, PartialEq)]
pub enum ADSRPhase {
    Attack,
    Decay,
    Sustain,
    Release,
    Silence,
}

#[derive(Clone)]
pub struct ADSRParam {
    a: f32,
    d: f32,
    s: f32,
    r: f32,
}

impl ADSRParam {
    pub fn new(a: f32, d: f32, s: f32, r: f32) -> Self {
        ADSRParam {
            a,
            d,
            s,
            r
        }
    }
}

pub struct ADSR {
    param: ADSRParam,
    note_on_duration: f32,
    note_off_duration: f32,
    current_val: f32,
    last_gate_val: f32,
    current_phase: ADSRPhase,
    current_event: ADSREvent,
    sample_rate: f32,
}

impl ADSR {
    pub fn new(sample_rate: f32) -> Self {
        ADSR {
            param: ADSRParam::new(0.0, 0.0, 1.0, 0.0),
            note_on_duration: 0.0,
            note_off_duration: 0.0,
            current_val: 0.0,
            last_gate_val: 0.0,
            current_phase: ADSRPhase::Silence,
            current_event: ADSREvent::NoteOff,
            sample_rate,
        }
    }

    pub fn set_param(&mut self, a: f32, d: f32, s: f32, r: f32) {
        self.param = ADSRParam::new(a, d, s, r);
    }

    fn next_phase(&self, next_event: ADSREvent) -> ADSRPhase {
        match next_event {
            ADSREvent::NoteOn => {
                let t = self.note_on_duration / self.sample_rate;
                if t < self.param.a {
                    ADSRPhase::Attack
                } else if self.param.a <= t && t < self.param.a + self.param.d {
                    ADSRPhase::Decay
                } else { // if self.a + self.d < t {
                    ADSRPhase::Sustain
                }
            },
            ADSREvent::NoteOff => {
                let t = self.note_off_duration / self.sample_rate;
                if t < self.param.r {
                    ADSRPhase::Release
                } else {
                    ADSRPhase::Silence
                }
            }
        }
    }

    fn retrigger(&mut self) {
        self.note_on_duration  = 0.0;
        self.note_off_duration = 0.0;
    }

    pub fn next(&mut self, next_event: ADSREvent) -> f32 {
        match next_event {
            ADSREvent::NoteOn => {
                if self.current_event == ADSREvent::NoteOff {
                    self.retrigger();
                }
                if self.current_phase != ADSRPhase::Sustain {
                    self.note_on_duration += 1.0;
                }
            },
            ADSREvent::NoteOff => {
                if self.current_event == ADSREvent::NoteOn {
                    self.last_gate_val = self.current_val; // remember last sample value before note off
                }
                if self.current_phase != ADSRPhase::Silence { // prevents incrementing note_off_duration infinitely
                    self.note_off_duration += 1.0;
                }
            }
        }
        
        self.current_event = next_event;
        self.current_phase = self.next_phase(next_event);
        self.current_val = match self.current_phase {
            ADSRPhase::Attack => {
                let t = self.note_on_duration / self.sample_rate / self.param.a;
                1.0 - (-5.0 * t).exp()
            },
            ADSRPhase::Decay => {
                let t = (self.note_on_duration / self.sample_rate - self.param.a) / self.param.d;
                self.param.s + (1.0 - self.param.s) * (-5.0 * t).exp()
            },
            ADSRPhase::Sustain => {
                self.param.s
            },
            ADSRPhase::Release => {
                let t = (self.note_off_duration + 1.0) / self.sample_rate / self.param.r;
                self.last_gate_val * (-5.0 * t).exp()
            },
            ADSRPhase::Silence => {
                0.0
            }
        };

        self.current_val
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
