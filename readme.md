# Supercollider Wrapper
This application manages a running instance of Supercollider (sclang, scsynth). OSC messages
	sent to its in-port will be forwarded to scsynth.

In order to enable more complete control over Supercollider it also provides custom functionality
	such as:
* A custom message for loading an scd synthdef string (for both realtime and NRT).
* A custom message for loading samples for simple playback (for both realtime and NRT).
* A node_id registry based on OSC-provided external ids, allowing e.g. changing of playing synth args without
    knowing nodeId.
    * With the option to provide the templated "{nodeId}" parameter, which fills the external id with the current nodeId to promote unique ids
* Functions for playing samples by name, note_on with timed note_off, streamlined NRT recording,
    and more.

The general aim is to provide easier ways to interact with Supercollider via OSC, for example
	when wanting to utilize core sclang functionality to create custom instruments or samplers.

Examples for calling all custom functions (via python) are provided in the "python" directory.

# Versions

Tested and working with
scsynth/sclang 3.13.0

# Notable "hacks" and issues to be aware of
- Important scd synth args
    - "buf" arg for sampler.scd is heavily referenced in sampler logic - biggest danger is attempting to supply it manually
    - "gate" arg is the universal "note off" arg - if a synthdef does not have "gate" logic no note off logic will work
- Insufficient lifecycle management
    - Many unhandled crashes or exits can still leave a running instance of sclang or scsynth
