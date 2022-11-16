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
    pub fn new(a: f32, d: f32, s: f32, r: f32, sample_rate: f32) -> Self {
        ADSR {
            param: ADSRParam::new(a, d, s, r),
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

    pub fn generate(&mut self, next_event: ADSREvent) -> f32 {
        let next_val = match next_event {
            ADSREvent::NoteOn => {
                if self.current_event == ADSREvent::NoteOff {
                    self.retrigger();
                }
                let next_val = self.next_val(self.next_phase(next_event));
                if self.current_phase != ADSRPhase::Sustain {
                    self.note_on_duration += 1.0;
                }
                next_val
            },
            ADSREvent::NoteOff => {
                if self.current_event == ADSREvent::NoteOn {
                    self.last_gate_val = self.current_val; // remember last sample value before note off
                }
                let next_val = self.next_val(self.next_phase(next_event));
                if self.current_phase != ADSRPhase::Silence {
                    self.note_off_duration += 1.0;
                }
                next_val
            }
        };

        self.current_event = next_event;
        self.current_phase = self.next_phase(next_event);
        self.current_val   = next_val;
        next_val
    }

    fn retrigger(&mut self) {
        self.note_on_duration  = 0.0;
        self.note_off_duration = 0.0;
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

    fn next_val(&self, next_phase: ADSRPhase) -> f32 {
        match next_phase {
            ADSRPhase::Attack => {
                let t = self.note_on_duration / self.sample_rate;
                if self.param.d > 0.0 {
                    1.0 / self.param.a * t
                } else {
                    self.param.s / self.param.a * t
                }
            },
            ADSRPhase::Decay => {
                let t = self.note_on_duration / self.sample_rate - self.param.a;
                1.0 - (1.0 - self.param.s) / self.param.d * t
            },
            ADSRPhase::Sustain => {
                self.param.s
            },
            ADSRPhase::Release => {
                let t = self.note_off_duration / self.sample_rate / self.param.r;
                self.last_gate_val - self.last_gate_val * t
            },
            ADSRPhase::Silence => {
                0.0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotters::prelude::*;
    use ADSREvent::*;
    use std::collections::VecDeque;

    fn create_chart(filename: &str, cap: &str, adsr: &mut ADSR, t_sec: f32, events: &mut VecDeque<(f32, ADSREvent)>) {
        let data_len: usize = (adsr.sample_rate * t_sec) as usize;
        let adsr_vec: Vec<f32> = (0..=data_len).map(|i| {
            let next_event = if events.is_empty() {
                adsr.current_event
            } else if events[events.len() - 1].0 > i as f32 / adsr.sample_rate {
                adsr.current_event
            } else {
                let e = events.pop_back().unwrap();
                e.1
            };
            adsr.generate(next_event)
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
}
