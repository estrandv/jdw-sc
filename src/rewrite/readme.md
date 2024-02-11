# I'm going to initiate a full rewrite of jdw-sc 

### Fundamental Design Changes
- If at all possible, it should manage only once process (not both sclang and scsynth)
    - sclang is only used for "read_scd"; it doesn't even have to be a single instance. Only needs to know port of server. 
    - start_server.scd might sitll couple things a bit too closely for this to be feasible. 
- No preloading of scripts 
    - "default" is a pefectly fine synth so we don't even need example on bootup for the welcome tune 
    - Synths should be sent by the user via another app 
    - NRT is trickier but ultimately based entirely on sclang as well
    - start_server.scd is in itself also an sclang operation 

### In general
- Slightly more data orientation and less struct-level variables (e.g. no supercollider.rs)

### jdw-scsynth 
- Handles convenience functions such as auto_note_off and external_id assignment
- Should be configured with process args to set port and other settings 
- Biggest challenge in having it standalone is the otherwise convenient waitForBoot 
    -> Can you do a health check via osc?
- /play_sample no longer really fits here 

### jdw-sclang 
- Essentially just a thin wrapper around lang to allow better osc-to-sclang commands 
    - You can't otherwise run freeform strings into sclang; it only takes files as startup scripts
- You then have two options: 
    1. Add some convenience functions, such as loading sample packs from disk and noting down the NRT state 
    2. OR let some other, python-based application do that
- The boring thing about the latter is that it ignores the initial purpose of the wrapper; to be a knowledge artifact for common sc scripting
    - As such it can be fun to allow things like full NRT record messages 
    - But again: we don't want to read samples on startup; a message should point to a dir 
- Revisiting samples
    - The Sampler synth need to play samples from buffers 
    - Both scsynth and nrt record thus need to assign each file to a Buffer.read call with an auto-incremented buffer number 
    - As such you can have functions that read sample dirs into memory and then auto-dump that into NRT if it happens ...
        ... but given everything else we might want to add on NRT record it would be better to allow some kind of pre-write 
    - The idea is that the NRT <messages> templating is a bit cumbersome to rewrite outside of here; it's nice to be able to 
        supply a bunch of timed packets and have that take on the appropriate format 
    
### Conclusion
- While the above is noble, things can very quickly get watered down into two base apps that do next to nothing 
    -> It's hard to decide on a scope when the purpose is mainly experimentation
- A good start, however, is to perform rewrites in here that simplify the code and keep the two processes as separate as possible 
    - This will allow us to rethink some of the gnarlier bits while working, like: 
        1. The sample dict structure, which feels more convoluted than it has to be 
        2. osc_model and internal_osc_conversion, which also appear to have a few to many layers 
        3. NRT_RECORD, which feels ill suited for appending things (like future effects)
        4. Just the reading of and responding to messages in general (a problem we've already solved with the poller in jdw-sequencer)