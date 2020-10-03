use vst::plugin_main;
use vst::plugin::{Plugin, Info, Category};
use fdn::FDN;
use vst::buffer::AudioBuffer;

struct FDNPlugin {
    fdn: FDN,
}

impl Plugin for FDNPlugin {
    fn get_info(&self) -> Info {
        Info {
            outputs: 1,
            inputs: 1,
            parameters: 0,
            name: "FDN".into(),
            category: Category::Spacializer,
            vendor: "SolarLiner".into(),
            unique_id: -4433,
            f64_precision: true,
            ..Default::default()
        }
    }

    fn process<'a>(&mut self, buffer: &mut AudioBuffer<'a, f32>) {
        let framesize = buffer.samples();
        self.fdn.update_framesize(framesize);

        let (inputs, mut outputs) = buffer.split();
        let input: Vec<_> = inputs[0].iter().cloned().map(|v| v as f64).collect();
        let mut output = vec![0.0; framesize];

        self.fdn.process(&mut output, &input);
        let output: Vec<_> = output.into_iter().map(|v| v as f32).collect();
        outputs[0].copy_from_slice(&output);
    }


    fn process_f64<'a>(&mut self, buffer: &mut AudioBuffer<'a, f64>) {
        self.fdn.update_framesize(dbg!(buffer.samples()));

        let (inputs, mut outputs) = buffer.split();
        let input = &inputs[0];
        let output = &mut outputs[0];

        self.fdn.process(output, input);
    }
}

impl Default for FDNPlugin {
    fn default() -> Self {
        FDNPlugin {
            fdn: FDN::new(4, 512),
        }
    }
}

plugin_main!(FDNPlugin);