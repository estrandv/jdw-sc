SynthDef.new("{:synth_name}",
{|amp=1, sus=1, pan=0, freq=340, hpf=200, ace=0.6, fcx=4, prt=0, bus=0,
attT=0.02, decT=0.0, susL=1.0, relT=0.0, gate=1, lfoS=0.0, lfoD=0.0, gain=1.0, fx=0.06|
var osc, snd, env, gen, filterenv, filter, lfosc, snd2, saw1, saw2, saw3;

amp = amp * gain;

// Portamento
freq = Lag.kr(freq, prt);

/*
    Actual brute has some form of RLPF (resonance/cutoff)
    This filter is then applied with adsr
*/

filterenv = Env.adsr(
attackTime: fcx,
sustainLevel: 0.2,
releaseTime: fcx * 4,
curve: -1.0,
peakLevel: 2
);

// Example of above: [0.0--->0.2->0.5------>1.0---->0.0]
// Current sound: When attack reaches max, we appear to drop to 0.0 amp for all freqs
// As we transfer to sustain, we return to some kind of amp (this is because fcs is 0.5, so the filter amount lowers!)
// WIth some more checks it would appear that the FILTER applies the same while AMP follows the adsr
// As such it appears that MUL does not concern itself with any separate freqs, it's just output in total
// I think the real solution is to have some form of crossfade
// So that naked SND is favoured over filter when ADSR is low

filterenv = EnvGen.kr(filterenv, gate, doneAction: Done.none);

lfosc = SinOsc.kr(lfoS).range((1 - lfoD), 1);

// Simple saw wave
saw1 = Saw.ar(freq: lfosc * freq, mul: amp, add: 0.0);
saw2 = Saw.ar(freq: lfosc * freq + (fx * 0.2), mul: amp, add: 0.0);
saw3 = Saw.ar(freq: lfosc * freq + (fx * 1.5), mul: amp, add: 0.0);

snd = saw1 + (saw2 * saw3);

snd = RHPF.ar(
in: snd,
freq: hpf * filterenv,
rq: ace * filterenv,
mul: 1.0
);

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

snd = snd * gen;

// Mono to stereo
snd = Mix(snd) * 0.5;

// Panning
snd = Pan2.ar(snd, pan);

Out.ar(bus, snd)}).{:operation}; // writeDefFile("synthdefs") or add