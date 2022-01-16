SynthDef.new("example", // Name pref the same as filename
{|amp=1, sus=1, pan=0, freq=440, bus=0| // osc args
var osc; // var defs

// Simple saw wave
osc = Saw.ar(freq: freq, mul: amp, add: 0.0);

// Mono to stereo
osc = Mix(osc) * 0.5;

Out.ar(bus, osc)}).{:operation}; // operation will be templated; not native sclang!