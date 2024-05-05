SynthDef.new("example",
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

Out.ar(bus, osc)})