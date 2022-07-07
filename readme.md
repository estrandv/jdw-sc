# Supercollider Wrapper
This is a wrapper application to manage a running instance of Supercollider server
	and sclang by forwarding and interpreting OSC messages. 
The main purposes of the project are as follows:
	1. Manage the lifecycle of and OSC-communication with the Supercollider and sclang processes
	2. Provide streamlined helper functions to simplify usage of more obtuse supercollider OSC functionality
		- e.g. s_new with timed sustain, NRT recording, sample playing
	3. Simplify the usage of Supercollider as a regular music instrument through:
		- A streamlined way to define sample packs and play them using OSC
		- A streamlined way to define synths for easy playing of single notes via OSC
		- A carefully structured and commented open source codebase
	4. Integrate Supercollider with the JackDAW microservice project via jdw-router