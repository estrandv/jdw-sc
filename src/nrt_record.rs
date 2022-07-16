/*
    TODO: Notes
    - With the current setup, we probably want to go via RunningNote -> to_s_new but with nrt format
    - create_note can thus be public... or we simply do it internally by giving node_manager an
        nrt function
    - Apart from that we need a general template from PROSC
    - Flow:
        1. A bundle for nrt arrives: [info, nrt_info, bundle:messages]
        2. Messages can be any of the main functions, ideally (sample, note_on, timed_gate...)
        3. A common trait for those will be needed in order to mass-convert into nrt format
            - Output should be vec
            - Will nodemanager be needed? We need an incrementer, but sc instance is irrelevant.
        4. General refactoring needed
            - If NodeManager isn't needed we should probably bypass RunningNote as well.
                - ... but there is no acute need to reinvent the wheel. All standard messages
                    can be converted to RunningNote and RunningNote can easily convert to nrt.
            - With RunningNote maintained we might only need to tweak the creation of each
                different msg->runningNote into something more generic
        5. Templating like in python. SampleDict must convert buffer-reads to initial scd.
        6. smooth sailing from here

    More musings
    - If a message can become runningNote it can become nrt
        - ... but nrt is just raw OSC and might not need to be a note variety
        - So we're sorta back at the idea of each "official" message having a to_nrt_format_osc
        - ... which has implications for both old and new code
    - Old code:
        - Incoming OSC needs to be converted into new OSC in a universal way
        - For example: passing s_new will just return the same message since it is not managed
            - ... but passing note_on will return the processed and converted osc
            - ... and note_on_timed will return more than one message with gate included
        - Perhaps I'm jumping too far.
            - We still want all our model structs as intermediaries for incoming data
            - Of course, we can still use poly to handle both managed and unmanaged messages
                    - a common trait get_osc and a default wrapper that just returns the original
            - It's still hard to fit runningnote into it all however.
                - I guess instead of keeping args you just convert straight away and keep osc
                - That way you can also kinda ditch the synth attribute
                - You do need id and external id however
                    - And it becomes a bit problematic when you want to convert on the fly,
                        e.g. note_mod... but then again no biggie since you can make a brand
                        new n_set.
        - Purpose of RunningNote
            - external_id -> node_id for n_set
            - Existence implies settable and running
            - ARgs are checked immediately on n_set to verify gate (otherwise only for msg trans)

        - What can we do?
            - Change RunningNote to contain OSC instead of args/synth
                - Maybe rename to ActiveOSC? Ah but not all OSC has nodeId...
            - Options for creating the OSC:
                1. Msg structs do it, taking in nodeManager or parts of it if needed
                    - n_set needs vector of running notes but otherwise fine
                    - s_new just needs id incrementer
                    - sampler needs sampledata
                2. NodeManager does it, taking in each specific message
                    - This couples them very tightly unless you split things out
                    -
            - Remove msg struct parsing from its trigger management
                - Each message can be converted to osc but won't necessarily have
                    the required other args for ActiveOSC (node_id, external_id).
                - It is thus better to first convert to OSC and then handle runningnote
                    as needed


         - The managed functions are a bit tricky
            - n_set assumes pre-existing with external id
                - ... but pre-conversion should be fine since non-running will just result in missing
                    anyway. So for each in order you keep track of added ids and resolve each n_set from that
            - ... others are fine I guess


         - Proposal: (eOSC = external, mMsg = Managed message, iOSC = internal, converted OSC)
            1. eOSC arrive and are converted to mMsg just like now (but without immediate send)
            2. Instead of NodeManager, a simple <external,nodeId> registry is kept in mainloop
                and passed to mMsg when calling conversion methods
            3. Conversion methods need some thinking for e.g. timed gate
                - If we want conversion to be a to_osc generic trait, we need the same info for all
                - If we want the timed gate built into the conversion, we need a timetag for
                    each OSC in the vector (wrapper?) which will be "0.0" for 99% of cases
                - We can also make each individual message have individual conversion methods
                    - So you would fetch the gate message separately and use gate_time to
                        tell the caller when to execute it
                    - This would however create issues with mass-conversion and sending.
                        Consider the bundle. What should "convert()" for each eOSC return?
                - What about to_nrt_osc?
                    - In addition to the idReg we'd need a curTime passed along
                    - ... and also fuck passing shit along
                    - Here the msg_time in a wrapper would be intuitive and useful however
                - So maybe it's all good, but sadly we will have to pass mutable things into
                    the conversion methods unless we add a lot of nifty thinking.
                    - It is also of course possible to put the conversion methods into whatever
                        object will manage <id> and <time> mutable states, but that will
                        fuck a bit with data ownership.
                    - Either way it gets a bit muddy. But it's probably best not to make
                        too big a fuzz about the <id> and <time> bits since they might need
                        to be arbitrary. I.e. for nrt you use a separate <id> registry
                        unrelated to the running one that you probably create on the spot.

            4. The remaining part of NodeManager (send functions) are ported to Supercollider
                struct.
                - For example: a function for sending the timed gate messages to server
                    in order. If messages have a time tag you can just send a plain vector
                    of time-tagged messages and use some kind of recursion for the thread creation.
                - One final problem we need to figure out before we begin:
                    - If gate_off is just an anonymous osc sent with a timer, how do we
                        know when to empty the <id> registry of expired notes?
                        - I mean... we could just... not? Problem I guess is infinite expansion.
                        - You could also add an optional expiration time and run a cleanup thread.
                            So a conversion method for s_new would add an optional while n_set
                            wouldn't... but here we are of course back in complex code territory
                            for the <id> registry.


            UPDATE:
                - We now have fully working message conversion with the new method
                - Registry cleanup is still not done - we can probably hack a timed release
                    for the specific handling of note_on_timed
                - Address string reading is still not generified for separate detection in the
                    nrt bundle. It's only three addresses so far... could be duplicate for all I care
                - Passing a separate node reg is no problem for the NRT batch conversion
                    - Pieces are in place!
                - Some other issues are coming up though:
                    - Might need to rethink time... is it the responsibility of the caller?
                        - Non-gate_time messages have no meaningful way to note spacing time,
                            which is more like the sequencer spacing.
                    - Main issue is the distinction between "message spacing" and "message delay"
                        - Our internal timed messages don't affect the timeline:
                            Two timed gate messages after each other should not wait for each other.
                            Then again that might not be as tricky as it seems.
                            [0.0, timed_gate, gt: 0.2] [0.1, timed_gate, gt: 0.1]
                            [[0.0, s_new][0.2, gate_off][0.1, s_new][0.2, gate_off]]
                            ... keep adding everything, then sort ...
                        - Problem, of course, is time increase selectivity
                            - I believe it is up to each conversion method to interpret what
                                moves the timeline and what does not
                            - THere is a built-in difference in format however
                                -> Bundle arrives with timed nrt messages
                                -> For each message, use either provided absolute time or
                                    relative time increase to bump timeline
                                -> Pass current timeline in when converting each unwrapped message
                                -> Gate times will be correctly seeded but have no impact
                        - As such the current setup is a good start!
                            - We just need a working format for messages that have a time tag
                                already on arrival
                            - ... which should ideally be the same standard for sequencer and
                                nrt to minimize format changes in callers
                            - ... which means we probably want time to be relative in the timed
                                messages, but otherwise there is no clashing
                            - A timed message in turn is... a bundle. Because it contains a message,
                                which can be anything.
                            - Timed message struct should thus be placed in the public osc library
                                and parsable from bundle
                            - [/bundle_info, "timed_msg"][/timed_msg, 0.0][/s_new...]

 */

use std::sync::{Arc, Mutex};

use rosc::OscType;

use crate::{IdRegistry, InternalOSCMorpher, NoteModifyMessage, NoteOnMessage, NoteOnTimedMessage, NRTRecordMessage, PlaySampleMessage, SampleDict};
use crate::osc_model::TimedOscMessage;
use crate::samples::Sample;

impl Sample {
    // Buffer load as-osc, suitable for loading into the NRT server
    pub fn to_nrt_scd_row(&self, dir: &str) -> String {
        // TODO: TEmplate-friendly pieces
        format!(
            "[0.0, (Buffer.new(server, 44100 * 8.0, 2, bufnum: {})).allocReadMsg(File.getcwd +/+ \"{}\")]",
            self.buffer_nr,
            dir.to_string() + "/" + &self.file_name.to_string(),
        )
    }
}

// TODO: Handle all the unwraps

impl NRTRecordMessage {
    pub fn get_processed_messages(
        &self,
        buffer_handle: Arc<Mutex<SampleDict>>,
    ) -> Vec<TimedOscMessage> {
        let registry = IdRegistry::new();
        let reg_handle = Arc::new(Mutex::new(registry));
        let mut current_time = 0.0;
        let mut result_vector: Vec<TimedOscMessage> = Vec::new();

        for msg in &self.messages {
            current_time += msg.time;

            // Each contained message must first be converted to its internal equivalent
            let rows =
                if msg.message.addr == "/note_on_timed" {
                    let res = NoteOnTimedMessage::new(&msg.message.clone());
                    res.unwrap()
                        .as_nrt_osc(reg_handle.clone(), current_time)
                } else if msg.message.addr == "/note_on" {
                    NoteOnMessage::new(&msg.message)
                        .unwrap()
                        .as_nrt_osc(reg_handle.clone(), current_time)
                } else if msg.message.addr == "/play_sample" {
                    let processed_message = PlaySampleMessage::new(&msg.message).unwrap();
                    processed_message.into_internal(
                        buffer_handle.clone()
                    ).as_nrt_osc(reg_handle.clone(), current_time)
                } else if msg.message.addr == "/note_modify" {
                    NoteModifyMessage::new(&msg.message)
                        .unwrap()
                        .as_nrt_osc(reg_handle.clone(), current_time)
                } else {
                    vec![] // TODO: Wrap in some default handler - important part is using current_time
                };

            current_time += msg.time;
            result_vector.extend(rows);
        }

        // Ensure all messages are in order
        result_vector.sort_by(|a, b| a.time.total_cmp(&b.time));

        result_vector
    }
}

impl TimedOscMessage {
    // Sort of a debug format; display as a string of values: [/s_new, "arg", 2.0, etc.]
    pub fn as_nrt_row(&self) -> String {
        let mut row_template = "[ {:time}, [\"{:adr}\",{:args}] ]".to_string();

        let args: Vec<_> = self.message.args.iter()
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
        row_template = row_template.replace("{:adr}", &format!("{}", &self.message.addr));

        let arg_string = args.join(",");

        row_template = row_template.replace("{:args}", &arg_string);

        row_template

    }
}

