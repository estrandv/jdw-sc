SynthDef.new("{:synth_name}",
{|amp=1, gain=1, sus=1, pan=0, freq=440, prt=0, bus=0, hpf=20, lpf=8000, attT=0.02, decT=0.0, susL=1.0, relT=0.0, phase=0.5, gate=1, lfoS=0.02, lfoD=0.0|
var osc, snd, env, gen;

amp = amp * gain;

freq = Lag.kr(freq, prt);

osc = FSinOsc.ar(freq: freq, iphase: phase, mul: amp, add: 0.0);

// ADSR
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

osc = osc * (SinOsc.ar(lfoS * freq).range((1 - lfoD), 1));

osc = osc * gen;

// Mono to stereo
snd = Mix(osc) * 0.5;

// High/Low pass filter
snd=HPF.ar(snd, hpf);
snd=LPF.ar(snd, lpf);

// Panning
snd = Pan2.ar(snd, pan);

Out.ar(bus, snd)}).{:operation};