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
]); // testcomment

a.recordNRT(
    outputFilePath: "temp/{:file_name}".standardizePath,
    headerFormat: "wav",
    sampleFormat: "int16",
    options: server.options,
    duration: {:end_time},
    // TODO: If it turns out we can call o here, o should be templated in
    // Update: Works!
    action: { {:out_socket_name}.sendMsg("/nrt_done", "ok"); }
);

server.remove;