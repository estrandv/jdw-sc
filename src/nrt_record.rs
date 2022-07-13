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
 */