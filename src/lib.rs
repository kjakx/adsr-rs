use dasp_signal::Signal;

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

#[derive(Copy, Clone, PartialEq)]
pub enum ADSRParamKind {
    AttackTime(f32),
    DecayTime(f32),
    SustainLevel(f32),
    ReleaseTime(f32),
    AttackCurve(f32),
    DecayCurve(f32),
    ReleaseCurve(f32),
}

impl ADSRParamKind {
    pub fn is_valid(self) -> bool {
        match self {
            ADSRParamKind::AttackTime(t) => {
                t >= 0.0
            },
            ADSRParamKind::DecayTime(t) => {
                t >= 0.0
            },
            ADSRParamKind::SustainLevel(l) => {
                l >= 0.0 && l <= 1.0
            },
            ADSRParamKind::ReleaseTime(t) => {
                t >= 0.0
            },
            ADSRParamKind::AttackCurve(c) => {
                c >= -1.0 && c <= 1.0
            },
            ADSRParamKind::DecayCurve(c) => {
                c >= -1.0 && c <= 1.0
            },
            ADSRParamKind::ReleaseCurve(c) => {
                c >= -1.0 && c <= 1.0
            },
        }
    }
}

#[derive(Clone)]
pub struct ADSRParams {
    attack_time   : f32,
    decay_time    : f32,
    sustain_level : f32,
    release_time  : f32,
    attack_curve  : f32,
    decay_curve   : f32,
    release_curve : f32,
}

impl ADSRParams {
    pub fn new(
        attack_time: f32, decay_time: f32, sustain_level: f32, release_time: f32,
        attack_curve: f32, decay_curve: f32, release_curve: f32
    ) -> Self {
        assert!(ADSRParamKind::AttackTime(attack_time).is_valid());
        assert!(ADSRParamKind::DecayTime(decay_time).is_valid());
        assert!(ADSRParamKind::SustainLevel(sustain_level).is_valid());
        assert!(ADSRParamKind::ReleaseTime(release_time).is_valid());
        assert!(ADSRParamKind::AttackCurve(attack_curve).is_valid());
        assert!(ADSRParamKind::DecayCurve(decay_curve).is_valid()); 
        assert!(ADSRParamKind::ReleaseCurve(release_curve).is_valid()); 

        ADSRParams {
            attack_time,
            attack_curve,
            decay_time,
            decay_curve,
            sustain_level,
            release_time,
            release_curve,
        }
    }

    pub fn set_param(&mut self, param: ADSRParamKind) {
        assert!(param.is_valid());
        match param {
            ADSRParamKind::AttackTime(t) => {
                self.attack_time = t;
            },
            ADSRParamKind::DecayTime(t) => {
                self.decay_time = t;
            },
            ADSRParamKind::SustainLevel(l) => {
                self.sustain_level = l;
            },
            ADSRParamKind::ReleaseTime(t) => {
                self.release_time = t;
            },
            ADSRParamKind::AttackCurve(c) => {
                self.attack_curve = c;
            },
            ADSRParamKind::DecayCurve(c) => {
                self.decay_curve = c;
            },
            ADSRParamKind::ReleaseCurve(c) => {
                self.release_curve = c;
            }
        }
    }
}

pub struct ADSR {
    params: ADSRParams,
    note_on_duration: f32,
    note_off_duration: f32,
    last_gate_val: f32,
    current_event: ADSREvent,
    current_phase: ADSRPhase,
    current_val: f32,
    next_event: ADSREvent,
    sample_rate: f32,
}

impl ADSR {
    pub fn new(a: f32, d: f32, s: f32, r: f32, sample_rate: f32) -> Self {
        ADSR {
            params: ADSRParams::new(a, d, s, r, 0.0, 0.0, 0.0),
            note_on_duration: 0.0,
            note_off_duration: 0.0,
            last_gate_val: 0.0,
            current_event: ADSREvent::NoteOff,
            current_phase: ADSRPhase::Silence,
            current_val: 0.0,
            next_event: ADSREvent::NoteOff,
            sample_rate,
        }
    }

    pub fn set_param(&mut self, param: ADSRParamKind) {
        self.params.set_param(param);
    }

    pub fn set_next_event(&mut self, event: ADSREvent) {
        self.next_event = event;
    }

    pub fn generate(&mut self) -> f32 {
        match self.next_event {
            ADSREvent::NoteOn => {
                if self.current_event == ADSREvent::NoteOff {
                    self.retrigger();
                }

                let next_phase = self.next_phase(self.next_event);
                let next_val = self.next_val(next_phase);

                if self.current_phase != ADSRPhase::Sustain {
                    self.note_on_duration += 1.0;
                }

                self.current_event = self.next_event;
                self.current_phase = next_phase;
                self.current_val   = next_val;
                next_val
            },
            ADSREvent::NoteOff => {
                if self.current_event == ADSREvent::NoteOn {
                    self.last_gate_val = self.current_val; // remember last sample value before note off
                }

                let next_phase = self.next_phase(self.next_event);
                let next_val = self.next_val(next_phase);

                if self.current_phase != ADSRPhase::Silence {
                    self.note_off_duration += 1.0;
                }

                self.current_event = self.next_event;
                self.current_phase = next_phase;
                self.current_val   = next_val;
                next_val
            }
        }
    }

    fn retrigger(&mut self) {
        self.note_on_duration  = 0.0;
        self.note_off_duration = 0.0;
    }

    fn next_phase(&self, next_event: ADSREvent) -> ADSRPhase {
        match next_event {
            ADSREvent::NoteOn => {
                let t = self.note_on_duration / self.sample_rate;
                if t < self.params.attack_time {
                    ADSRPhase::Attack
                } else if t < self.params.decay_time + self.params.attack_time {
                    ADSRPhase::Decay
                } else { // if attack_time + decay_time <= t {
                    ADSRPhase::Sustain
                }
            },
            ADSREvent::NoteOff => {
                let t = self.note_off_duration / self.sample_rate;
                if t < self.params.release_time {
                    ADSRPhase::Release
                } else {
                    ADSRPhase::Silence
                }
            }
        }
    }

    fn next_val(&self, next_phase: ADSRPhase) -> f32 {
        match next_phase {
            ADSRPhase::Attack => {
                let t = self.note_on_duration / self.sample_rate;
                if self.params.decay_time > 0.0 {
                    Self::curve_function(t, 1.0, self.params.attack_time, self.params.attack_curve)
                } else {
                    Self::curve_function(t, self.params.sustain_level, self.params.attack_time, self.params.attack_curve)
                }
            },
            ADSRPhase::Decay => {
                let t = self.note_on_duration / self.sample_rate - self.params.attack_time;
                Self::curve_function(self.params.decay_time - t, 1.0 - self.params.sustain_level, self.params.decay_time, self.params.decay_curve) + self.params.sustain_level
            },
            ADSRPhase::Sustain => {
                self.params.sustain_level
            },
            ADSRPhase::Release => {
                let t = self.note_off_duration / self.sample_rate;
                Self::curve_function(self.params.release_time - t, self.last_gate_val, self.params.release_time, self.params.release_curve)
            },
            ADSRPhase::Silence => {
                0.0
            }
        }
    }

    // exponential curve that passes (0, 0) and (w, h)
    fn curve_function(x: f32, h: f32, w: f32, curve_factor: f32) -> f32 {
        assert!(x >= 0.0);
        assert!(h >= 0.0);
        assert!(w > 0.0);
        assert!(curve_factor >= -1.0 && curve_factor <= 1.0);
        if curve_factor == 0.0 { // linear
            h / w * x
        } else {
            const EPS: f32 = 0.005;
            let r = -curve_factor * (0.5 - EPS) + 0.5; // -1.0..1.0 -> 1.0-eps..0.0+eps
            h*((1.0/r-1.0).powf(2.0*x/w)-1.0)/((1.0/r-1.0).powf(2.0)-1.0)
        }
    }
}

impl Signal for ADSR {
    type Frame = f32;

    fn next(&mut self) -> Self::Frame {
        self.generate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotters::prelude::*;
    use ADSREvent::*;
    use ADSRParamKind::*;
    use std::collections::VecDeque;

    fn create_chart(filename: &str, cap: &str, adsr: &mut ADSR, t_sec: f32, events: &mut VecDeque<(f32, ADSREvent)>) {
        let data_len: usize = (adsr.sample_rate * t_sec) as usize;
        let adsr_vec: Vec<f32> = (0..=data_len).map(|i| {
            if !events.is_empty() && events[events.len() - 1].0 <= i as f32 / adsr.sample_rate {
                let e = events.pop_back().unwrap();
                adsr.set_next_event(e.1);
            }
            adsr.next()
        }).collect();

        let root = BitMapBackend::new(filename, (1024, 768)).into_drawing_area();

        root.fill(&WHITE).unwrap();

        let mut chart = ChartBuilder::on(&root)
            .set_label_area_size(LabelAreaPosition::Left, 60)
            .set_label_area_size(LabelAreaPosition::Bottom, 60)
            .caption(cap, ("sans-serif", 40))
            .build_cartesian_2d(-0.5_f32..t_sec+1.0, -0.1_f32..1.1_f32)
            .unwrap();

        chart
            .configure_mesh()
            //.disable_x_mesh()
            //.disable_y_mesh()
            .draw()
            .unwrap();

        chart.draw_series(
            AreaSeries::new(
                (0..=data_len).zip(adsr_vec.iter()).map(|(x, y)| ((x as f32 / adsr.sample_rate, *y))),
                0.0,
                &RED.mix(0.2),
            )
            .border_style(&RED),
        ).unwrap();

        // To avoid the IO failure being ignored silently, we manually call the present function
        root.present().unwrap();
    }

    #[test]
    fn silence() {
        let mut event_queue: VecDeque<(f32, ADSREvent)> = VecDeque::new();
        let mut adsr = ADSR::new(0.1, 0.1, 0.1, 0.1, 100.0);
        create_chart("chart/silence.png", "silence", &mut adsr, 2.0, &mut event_queue)
    }

    #[test]
    fn silence_although_note_on() {
        let mut event_queue = VecDeque::new();
        event_queue.push_front((0.1, NoteOn));
        let mut adsr = ADSR::new(0.0, 0.0, 0.0, 0.0, 100.0);
        create_chart("chart/silence_although_note_on.png", "silence_although_note_on", &mut adsr, 2.0, &mut event_queue);
    }

    #[test]
    fn just_sustain() {
        let mut event_queue = VecDeque::new();
        event_queue.push_front((0.0, NoteOn));
        let mut adsr = ADSR::new(0.0, 0.0, 1.0, 0.0, 100.0);
        create_chart("chart/just_sustain.png", "just_sustain", &mut adsr, 2.0, &mut event_queue);
    }

    #[test]
    fn typical() {
        let mut event_queue = VecDeque::new();
        event_queue.push_front((0.0, NoteOn));
        event_queue.push_front((1.0, NoteOff));
        let mut adsr = ADSR::new(0.2, 0.2, 0.8, 1.0, 100.0);
        adsr.set_param(AttackCurve(-0.5));
        adsr.set_param(DecayCurve(0.4));
        adsr.set_param(ReleaseCurve(0.6));
        create_chart("chart/typical.png", "typical", &mut adsr, 2.0, &mut event_queue);
    }

    #[test]
    fn fade_in_out() {
        let mut event_queue = VecDeque::new();
        event_queue.push_front((0.0, NoteOn));
        event_queue.push_front((1.5, NoteOff));
        let mut adsr = ADSR::new(0.5, 0.0, 0.8, 0.5, 100.0);
        create_chart("chart/fade_in_out.png", "fade_in_out", &mut adsr, 2.0, &mut event_queue);
    }

    #[test]
    fn curvature_edge_case() {
        let mut event_queue = VecDeque::new();
        event_queue.push_front((0.0, NoteOn));
        event_queue.push_front((1.5, NoteOff));
        let mut adsr = ADSR::new(0.5, 0.5, 0.5, 0.5, 100.0);
        adsr.set_param(AttackCurve(-1.0));
        adsr.set_param(DecayCurve(0.0));
        adsr.set_param(ReleaseCurve(1.0));
        create_chart("chart/curvature_edge_case.png", "curvature_edge_case", &mut adsr, 2.0, &mut event_queue);
    }
}
