var server, buf;

TempoClock.default.tempo = {:bpm}/60;

server = Server(\nrt,
    options: ServerOptions.new
    .numOutputBusChannels_(2)
    .numInputBusChannels_(2)
);

//server.latency = 0.05;

buf = Buffer.new(server, 0, 1);

a = Score([
    {:score_rows}
]);

a.recordNRT(
    outputFilePath: "temp/{:file_name}".standardizePath,
    headerFormat: "wav",
    sampleFormat: "int16",
    options: server.options,
    duration: {:end_time},
    action: { File.getcwd().postln }
);

server.remove;