/*
    Server launch script, containing jdw application specific configurations and custom functions.
    Note that this is a template - {:var} placeholders are filled in scd_templating.rs
    TODO: using o to send messages into main logger would make for a better alternative to println
*/

//Server.supernova; // Can be used to switch backend.
s = Server.new("jdw-sc", NetAddr.new("127.0.0.1", 13337));
s.options.memSize = 2000000;
// Server latency only affects PBind, which in turn uses it by placin it in the <time> arg of bundled sends to server
// Leaving it here for a bit as a reminder that it is useless
//s.latency = 0.2;

// https://crashserver.fr/tutorial-how-to-record-foxdot-multi-outputs-in-a-daw/
// s.options.numOutputBusChannels = 8;

o = NetAddr.new("127.0.0.1", 13338);

//History.showLogFile;
s.waitForBoot({
	("Client Port: " + NetAddr.langPort).postln;

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

    /*
        Custom function for reading scd strings from files.
        Useful when /read_scd can't handle the size of the scd string you want to send.
    */
    OSCFunc({
        arg msg, time, addr, port;
        var file;
        file = File.open(msg[1].asString, "r");
    	"Reading scd file: " + msg[1].postln;
        file.readAllString().interpret();
        file.close();
    }, '/read_scd_file');


	/*
	    Custom message sent on startup completion to let the wrapper know that all custom logic is ready
	*/
	o.sendMsg("/init", "ok");
});
