use fdn::FDN;
use hound::{WavIntoSamples, WavReader, WavWriter};
use itertools::Itertools;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "Wave Processor", about = "Process wav files through the FDN")]
struct Options {
    #[structopt(name = "INPUT")]
    input: PathBuf,
    #[structopt(name = "OUTPUT")]
    output: PathBuf,
}

const FRAME_SIZE: usize = 256;

fn main() {
    let opts: Options = Options::from_args();
    let mut reader = WavReader::open(opts.input).unwrap();
    let mut spec = reader.spec();
    let mut writer = WavWriter::create(opts.output, spec).unwrap();
    let mut fdn = FDN::new(1, FRAME_SIZE);
    let samples = reader.samples::<f32>();
    for chunk in &samples.flatten().map(|s| s as f64).chunks(FRAME_SIZE) {
        let mut out = vec![0.0; FRAME_SIZE];
        let mut chunk: Vec<f64> = chunk.into_iter().map(Into::into).collect();
        if chunk.len() < FRAME_SIZE {
            chunk.extend(std::iter::repeat(0.0).take(FRAME_SIZE - chunk.len()));
        }
        fdn.process(&mut out, &chunk);

        for s in out {
            writer.write_sample(s as f32).unwrap();
        }
    }

    writer.finalize().unwrap();
}
