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

 */