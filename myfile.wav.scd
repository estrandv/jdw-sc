var server, buf, opts;

TempoClock.default.tempo = 120/60;

opts = ServerOptions.new;
opts.numOutputBusChannels = 2;
opts.numInputBusChannels = 2;
opts.sampleRate = 48000;

server = Server(\nrt,
    options: opts
);

//server.latency = 0.05;

// TODO: Pretty sure buffers are added in score now - but this is how a bus would be added, too! 
buf = Buffer.new(server, 0, 1);

a = Score([
    [0.0, ['/d_recv', // SynthDef for playing samples by buffer number
SynthDef("sampler", { |out = 0, start = 0, sus = 10, amp = 1, rate = 1, buf = 0, pan = 0, ofs=0.05|
    var osc = PlayBuf.ar(1, buf, BufRateScale.kr(buf) * rate, startPos: start);
    amp = amp * 2.0; // I have found that sample amp usually lands way lower than any synth amp
    osc = osc * EnvGen.ar(Env([0,1 * amp,1 * amp,0],[ofs, sus-0.05, 0.05]), doneAction: Done.freeSelf);
    osc = Mix(osc);
    osc = Pan2.ar(osc, pan);
	Out.ar(out, osc)
}).asBytes]],
[0.0, ['/d_recv', SynthDef.new("example",
{|amp=1, sus=1, pan=0, freq=440, bus=0, attT=0, decT=0,susL=1, relT=1, gate=1| // osc args
var osc, env, gen; // var defs

// Simple sine
osc = Pulse.ar(freq: freq, mul: amp, add: 0.0);

// ADSR reacts to gate:1/0 as "note off" and allows the synth to fade after gate=0 is sent 
// It is recommended to use some kind of gate system for all new synths to allow automatic turn-off
env = Env.adsr(
    attackTime: attT,
    decayTime: decT,
    sustainLevel: susL,
    releaseTime: relT,
    peakLevel: 1.0,
    curve: -4.0,
    bias: 0.0
);

gen = EnvGen.kr(env, gate, doneAction: Done.freeSelf);

// Mono to stereo
osc = osc * gen; 
osc = Mix(osc) * 0.5;
osc = Pan2.ar(osc, 0.0);

Out.ar(bus, osc)}).asBytes]],
[0.0, (Buffer.new(server, 44100 * 8.0, 2, bufnum: 100)).allocReadMsg("/home/estrandv/programming/jdw-sc/python/wav/snare.wav")],
[0.0, (Buffer.new(server, 44100 * 8.0, 2, bufnum: 100)).allocReadMsg("/home/estrandv/programming/jdw-sc/python/wav/snare.wav")],
[ 0.00000, ["/s_new","example",101,0,0,"freq",130.00000] ],
[ 0.10000, ["/n_set",101,"gate",0.00000] ],
[ 1.00000, ["/s_new","example",102,0,0,"freq",160.00000] ],
[ 2.10000, ["/n_set",102,"gate",0.00000] ],
[ 2.00000, ["/s_new","example",103,0,0,"freq",143.00000] ],
[ 2.50000, ["/n_set",103,"gate",0.00000] ],
[ 2.00000, ["/s_new","sampler",104,0,0,"amp",1.00000,"ofs",0.00000,"buf",100] ],
[ 2.25000, ["/s_new","sampler",105,0,0,"amp",1.00000,"ofs",0.00000,"buf",100] ]
]); // testcomment

a.recordNRT(
    outputFilePath: "myfile.wav".standardizePath,
    headerFormat: "wav",
    sampleFormat: "int16",
    options: server.options,
    duration: 6,
    action: { o.sendMsg("/nrt_done", "ok"); }
);

server.remove;