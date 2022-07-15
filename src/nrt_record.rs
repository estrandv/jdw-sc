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


 */