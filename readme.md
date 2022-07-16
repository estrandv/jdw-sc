# Supercollider Wrapper
This is a wrapper application to manage a running instance of Supercollider server
	and sclang by forwarding and interpreting OSC messages. 
The main purposes of the project are as follows:
	1. Manage the lifecycle of and OSC-communication with the Supercollider and sclang processes
	2. Provide streamlined helper functions to simplify usage of more obtuse supercollider OSC functionality
		- e.g. s_new with timed sustain, NRT recording, sample playing by name/category, n_set
			based on external id tags provided via OSC
	3. Simplify the usage of Supercollider as a regular music instrument through:
		- A streamlined way to define synths for easy playing of and modification of single notes via OSC
        - A streamlined way to define sample packs and play them using OSC
		- A carefully structured and commented open source codebase
			- Including example scripts for calling all custom functions

# Notable "hacks" to keep track of
- Important scd synth args
    - "bus" arg for sampler.scd is heavily referenced in sampler logic - biggest danger is attempting to supply it manually
    - "gate" arg is the universal "note off" arg - if a synthdef does not have "gate" logic no note off logic will work
- Insufficient lifecycle management
    - Many unhandled crashes or exits can still leave a running instance of sclang or scsynth
