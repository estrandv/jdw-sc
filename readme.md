# Supercollider Wrapper
This application manages a running instance of Supercollider (sclang, scsynth). OSC messages
	sent to its in-port will be forwarded to scsynth.

In order to enable more complete control over Supercollider it also provides custom functionality
	such as:	
	- A preloaded directory of user-defined synthdefs for easy synth creation
	- A predefined structure for creating buffer-playable sample-packs as directories of samples
	- A node_id registry based on OSC-provided external ids, allowing e.g. changing of playing synth args without
		knowing nodeId
	- Functions for playing samples by name, note_on with timed note_off, streamlined NRT recording,
		and more. 

The general aim is to provide easier ways to interact with Supercollider via OSC, for example
	when wanting to utilize core sclang functionality to create custom instruments or samplers.

Examples for calling all custom functions (via python) are provided in the "python" directory.

# Notable "hacks" to keep track of
- Important scd synth args
    - "bus" arg for sampler.scd is heavily referenced in sampler logic - biggest danger is attempting to supply it manually
    - "gate" arg is the universal "note off" arg - if a synthdef does not have "gate" logic no note off logic will work
- Insufficient lifecycle management
    - Many unhandled crashes or exits can still leave a running instance of sclang or scsynth
