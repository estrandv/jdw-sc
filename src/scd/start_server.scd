//Server.supernova; // Can be used to switch backend.
s = Server.new("prosc", NetAddr.new("127.0.0.1", 13337));
s.options.memSize = 2000000; // 2gb
s.latency = 0.05; // Default latency setting is 0.2 sec - this might be irrelevant for our use case but set nontheless

// https://crashserver.fr/tutorial-how-to-record-foxdot-multi-outputs-in-a-daw/
// s.options.numOutputBusChannels = 8;

o = NetAddr.new("127.0.0.1", 13338);

//History.showLogFile;
s.waitForBoot({
	("Client Port: " + NetAddr.langPort).postln;

	/*
		Custom function for calling scd scripts via osc during runtime.

		MSG: customScript, <scd_file_path>
	*/
	OSCFunc(
		{
			arg msg, time, addr, port;
			var file_path, fn;

			// Get local filename

			file_path = msg[1].asString;

			fn = File(file_path, "r");
			fn.readAllString.interpret;
			fn.close;

			o.sendMsg("/customscript_done", "ok");

		},
		'/customscript'
	);

	/*
    		Custom function for flat out reading plain scd strings

    		MSG: read_scd, <scd string>
    */
    OSCFunc(
        {
            arg msg, time, addr, port;
            msg[1].asString.interpret;
            o.sendMsg("/read_scd_done", "ok");
        },
        '/read_scd'
    );

	//"Ready!".postln;

	/*
	    Custom message sent on startup completion to let the wrapper know that all custom logic is ready
	*/
	o.sendMsg("/init", "ok");
});

