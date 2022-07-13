// SynthDef for playing samples by buffer number - see samples.rs for buffer methodology
SynthDef("{:synth_name}", { |bus = 0, start = 0, sus = 10, amp = 1, rate = 1, buf = 0, pan = 0, ofs=0.05|
    var osc = PlayBuf.ar(buf.numChannels, buf, BufRateScale.kr(buf) * rate, startPos: start);
    osc = osc * EnvGen.ar(Env([0,1 * amp,1 * amp,0],[ofs, sus-0.05, 0.05]), doneAction: Done.freeSelf);
    osc = Mix(osc);
    osc = Pan2.ar(osc, pan);
	Out.ar(bus, osc)
}).{:operation};
