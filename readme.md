# JackDAW Supercollider
- SCLang and SCSynth process management
	- OSC library: ROSC has good examples 
	- Async/Thread listener for server callback 
- Structs
	- All incoming messages (or can we do them as maps..?) 
		- Would it be worth it to do some kind of polymorphism? 
		- Like... with common ZMQMessage behaviour like handle and contents? 
	- Outgoing OSC
		- Any wrappers needed, or should we just construct in-function and send?
	- RunningNote impl 
- Templates
	- Could probably outline as structs with to_string() 
	- NRT callback? 
- Cleanup thread
- ZMQ incoming read thread 
- Buffer Data 

### FUture plans
- Everything as OSC? 
	- https://www.music.mcgill.ca/~gary/306/week9/osc.html
	- OSC UDP is FAST 
	- "address" usage can replace the wonky JDW.NOTE.PLAY:: syntax of current 
	- Through some kind of bundle stacking we can achieve the sequencer requirements of 
		<tag, time, contained_message> through som kind of 
		bundle:<msg:<tag, time>, other_msg:<...>>
		- Only issue is pattern matching / filtering
