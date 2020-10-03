use ringbuf::{Consumer, Producer, RingBuffer};
use std::ops::{AddAssign, Mul, Add};

pub struct DelayLine {
    rx: Consumer<f64>,
    tx: Producer<f64>,
    steady_state: bool,
}

impl DelayLine {
    pub fn new(delaysamples: usize) -> Self {
        let ringbuf = RingBuffer::new(delaysamples);
        let (tx, rx) = ringbuf.split();
        Self {
            tx,
            rx,
            steady_state: false,
        }
    }

    pub fn process(&mut self, out: &mut [f64], inp: &[f64]) {
        debug_assert_eq!(out.len(), inp.len());

        for (o, &i) in out.iter_mut().zip(inp.iter()) {
            *o = 0.0;
            if self.steady_state {
                *o = self.rx.pop().unwrap();
                self.tx.push(i).unwrap();
            } else {
                if self.tx.is_full() {
                    *o = self.rx.pop().unwrap();
                    self.tx.push(i);
                    self.steady_state = true;
                } else {
                    self.tx.push(i).unwrap();
                }
            }
        }
    }
}

pub struct PhiIterator {
    cur: f64,
}

impl Default for PhiIterator {
    fn default() -> Self {
        Self { cur: 1.0 }
    }
}

impl Iterator for PhiIterator {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        self.cur *= PHI;
        Some(self.cur)
    }
}

pub struct FDN {
    fb_mat: [[f64; 4]; 4],
    delay: [DelayLine; 4],
    delay_fb: [DelayLine; 4],
    feedback: [Vec<f64>; 4],
}

const PHI: f64 = 1.6180339887;

impl FDN {
    pub fn new(ndelays: usize, frame_size: usize) -> Self {
        let delaysamples = [800, 1422, 2483, 4227];
        Self {
            fb_mat: [
                [0.0, 1.0, 1.0, 0.0],
                [-1.0, 0.0, 0.0, -1.0],
                [1.0, 0.0, 0.0, -1.0],
                [0.0, 1.0, -1.0, 0.0],
            ],
            delay: [
                DelayLine::new(delaysamples[0]),
                DelayLine::new(delaysamples[1]),
                DelayLine::new(delaysamples[2]),
                DelayLine::new(delaysamples[3]),
            ],
            delay_fb: [
                DelayLine::new(delaysamples[0].checked_sub(frame_size).unwrap_or(1)),
                DelayLine::new(delaysamples[1].checked_sub(frame_size).unwrap_or(1)),
                DelayLine::new(delaysamples[2].checked_sub(frame_size).unwrap_or(1)),
                DelayLine::new(delaysamples[3].checked_sub(frame_size).unwrap_or(1)),
            ],
            feedback: [
                vec![0.0; frame_size],
                vec![0.0; frame_size],
                vec![0.0; frame_size],
                vec![0.0; frame_size],
            ],
        }
    }

    pub fn update_framesize(&mut self, framesize: usize) {
        for fb in self.feedback.iter_mut() {
            fb.resize(framesize, 0.0);
        }
    }

    pub fn process(&mut self, out: &mut [f64], inp: &[f64]) {
        zero(out);
        let nframes = self.feedback[0].len();

        let mut inp_local: Vec<_> = inp.into();
        let mut inp_local = [
            inp_local.clone(),
            inp_local.clone(),
            inp_local.clone(),
            inp_local,
        ];

        // On mélange les signaux de feedback entre eux
        for out_idx in 0..4 {
            for in_idx in 0..4 {
                for (o, i) in inp_local[out_idx]
                    .iter_mut()
                    .zip(self.feedback[in_idx].iter())
                {
                    *o += *i * self.fb_mat[out_idx][in_idx] * 0.65;
                }
            }
        }

        // On applique le retard a la sortie directe
        for (delay, inp) in self.delay.iter_mut().zip(inp_local.iter()) {
            let mut tout = vec![0.0; nframes];
            delay.process(&mut tout, inp);

            elementwise_add(out, &tout);
        }

        // On applique le retard au feedback
        for ((delayfb, inp), feedback) in self
            .delay_fb
            .iter_mut()
            .zip(inp_local.iter())
            .zip(self.feedback.iter_mut())
        {
            delayfb.process(feedback, inp);
        }

        //mix(out, inp, 0.1);
    }
}

fn copy<T: Clone>(to: &mut [T], from: &[T]) {
    for (o, i) in to.iter_mut().zip(from.iter()) {
        *o = i.clone();
    }
}

fn elementwise_add<T: AddAssign + Clone>(into: &mut [T], from: &[T]) {
    for (o, i) in into.iter_mut().zip(from.iter()) {
        *o += i.clone();
    }
}

fn mix(into: &mut [f64], from: &[f64], t: f64) {
    for (o, i) in into.iter_mut().zip(from.iter()) {
        *o = *o * t + *i * (1.0 - t);
    }
}

fn gen_matrix(ndelays: usize, val: f64) -> Vec<Vec<f64>> {
    let mut out = vec![vec![0.0; ndelays]; ndelays];

    for i in 0..ndelays {
        out[(i + 1) % ndelays][i] = val;
        out[(i + ndelays - 2) % ndelays][i] = -val;
    }

    return out;
}

fn zero<T: Default>(arr: &mut [T]) {
    arr.iter_mut().for_each(|v| *v = T::default());
}

fn hadamard(logn: usize) -> Vec<Vec<isize>> {
    match logn {
        0 => vec![],
        1 => vec![vec![1]],
        n => {
            let lower = hadamard(n - 1);
            let len = 2usize.pow((n - 1) as u32);
            let mut out = vec![Vec::with_capacity(len); len];
            for (i, line) in lower.iter().enumerate() {
                out[i].extend_from_slice(line);
                out[i].extend_from_slice(line);
                out[i + lower.len()].extend_from_slice(line);
                out[i + lower.len()].extend(line.iter().cloned().map(|v| -v));
            }

            out
        }
    }
}

fn trunc_matrix<T: Clone>(data: Vec<Vec<T>>, m: usize, n: usize) -> Vec<Vec<T>> {
    data.into_iter()
        .map(|i| i.into_iter().take(n).collect())
        .take(m)
        .collect()
}

#[test]
fn test_hadamard_2() {
    let res = vec![vec![1, 1], vec![1, -1]];
    let act = hadamard(2);

    assert_eq!(res, act);
}

#[test]
fn test_hadamard_4() {
    let res = vec![
        vec![1, 1, 1, 1],
        vec![1, -1, 1, -1],
        vec![1, 1, -1, -1],
        vec![1, -1, -1, 1],
    ];
    let act = hadamard(3);

    assert_eq!(res, act);
}

#[test]
fn test_trun_mat_has_correct_size() {
    let mat = vec![vec![1.0; 10]; 10];
    let res = trunc_matrix(mat, 4, 5);

    assert_eq!(4, res.len());
    assert_eq!(5, res[0].len());
}
