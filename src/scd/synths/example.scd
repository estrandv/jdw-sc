SynthDef.new("{:synth_name}", // Name will be same as filename without .scd extension
{|amp=1, sus=1, pan=0, freq=440, bus=0| // osc args
var osc; // var defs

// Simple saw wave
osc = Saw.ar(freq: freq, mul: amp, add: 0.0);

// Note that there is no gate/env, so this synth cannot be turned off the traditional way ...

// Mono to stereo
osc = Mix(osc) * 0.5;

Out.ar(bus, osc)}).{:operation};

// Note: {:word} format is not sclang but an arbitrary templating format used for placeholders
// See: synth_templates.rs