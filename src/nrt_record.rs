use jdw_osc_lib::model::TimedOSCPacket;
use rosc::{OscPacket, OscType};

pub trait NRTConvert {
    fn as_nrt_row(&self) -> String;
}

impl NRTConvert for TimedOSCPacket {
    // Sort of a debug format; display as a string of values: [/s_new, "arg", 2.0, etc.]
    fn as_nrt_row(&self) -> String {
        let mut row_template = "[ {:time}, [\"{:adr}\",{:args}] ]".to_string();

        // TODO: Gonna cheat here for now. We're supposed to do the whole processor routine...
        let msg = match &self.packet {
            OscPacket::Message(msg) => { Some(msg.clone()) }
            OscPacket::Bundle(_) => { None }
        };

        let args: Vec<_> = msg.clone().unwrap().args.iter()
            .map(|arg| {
                let ball = match arg {
                    OscType::Int(val) => {
                        format!("{}", val)
                    }
                    OscType::Float(val) => {
                        format!("{:.5}", val)
                    }
                    OscType::String(val) => {
                        format!("\"{}\"", val)
                    }
                    _ => {
                        // TODO: Implement everything some day
                        "err".to_string()
                    }
                };
                ball
            }).collect();

        row_template = row_template.replace("{:time}", &format!("{:.5}", &self.time));
        row_template = row_template.replace("{:adr}", &format!("{}", msg.unwrap().addr));

        let arg_string = args.join(",");

        row_template = row_template.replace("{:args}", &arg_string);

        row_template
    }
}

