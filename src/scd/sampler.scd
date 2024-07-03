// SynthDef for playing samples by buffer number
SynthDef("sampler", { |bus = 0, start = 0, sus = 10, amp = 1, rate = 1, buf = 0, pan = 0, ofs=0.05|
    var osc = PlayBuf.ar(1, buf, BufRateScale.kr(buf) * rate, startPos: start);
    amp = amp * 2.0; // I have found that sample amp usually lands way lower than any synth amp
    osc = osc * EnvGen.ar(Env([0,1 * amp,1 * amp,0],[ofs, sus-0.05, 0.05]), doneAction: Done.freeSelf);
    osc = Mix(osc);
    osc = Pan2.ar(osc, pan);
	Out.ar(bus, osc)
})