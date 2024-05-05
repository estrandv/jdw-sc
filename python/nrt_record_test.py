
from pythonosc import udp_client
from pythonosc import osc_bundle_builder
from pythonosc import osc_message_builder

# Hardcoded default port of jdw-sc main application
client = udp_client.SimpleUDPClient("127.0.0.1", 13331)

# Construct a bundle containing:
#   (a) nrt_record as /bundle_info message (tagged bundle)
#   (b) /nrt_record_info second message with args [bpm, file_name, end_beat]
#   (c) raw bundle of timed_message bundles (each containing a timed_message info tag and an osc message)
msg = osc_message_builder.OscMessageBuilder(address="/bundle_info")
msg.add_arg("nrt_record")
bundle = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)
bundle.add_content(msg.build())
msg2 = osc_message_builder.OscMessageBuilder(address="/nrt_record_info")
msg2.add_arg(120.0)
msg2.add_arg("myfile.wav")
msg2.add_arg(6.0) # A smarter program would adjust this according to timed messages added (end beat)
bundle.add_content(msg2.build())

# Ensure a synth and sample pack 
import os
wav_file = os.path.dirname(os.path.realpath(__file__)) + "/wav/snare.wav"
client.send_message("/load_sample", [wav_file, "example", 100, "bd"])
with open("synths/example.scd", "r") as synthdef:
    client.send_message("/create_synthdef", synthdef.read())

rows_bundle = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)

def create_timed_message(time, osc_msg):
    bun = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)
    top_msg = osc_message_builder.OscMessageBuilder(address="/bundle_info")
    top_msg.add_arg("timed_msg")
    info_msg = osc_message_builder.OscMessageBuilder(address="/timed_msg_info")
    info_msg.add_arg(time)
    bun.add_content(top_msg.build())
    bun.add_content(info_msg.build())
    bun.add_content(osc_msg.build())
    return bun.build()

def make_note(time, args):
    args = ["example", "gentle_nrt_id"] + args
    note_msg = osc_message_builder.OscMessageBuilder(address="/note_on_timed")
    for arg in args:
        note_msg.add_arg(arg)

    timed_msg = create_timed_message(time, note_msg)
    rows_bundle.add_content(timed_msg)

def make_drum(time):
    # NOTE: Since we have no built-in sample packs, this will only work if you have an example pack in /home
    args = ["nsam_id", "example", 0, "sn", 0, "amp", 1.0, "ofs", 0.0]
    note_msg = osc_message_builder.OscMessageBuilder(address="/play_sample")
    for arg in args:
        note_msg.add_arg(arg)
    timed_msg = create_timed_message(time, note_msg)
    rows_bundle.add_content(timed_msg)

# First args is reserved time, rather than placement on timeline
make_note("1.0", ["0.1", 0, "freq", 130.0])
make_note("1.0", ["1.1", 0, "freq", 160.0])
make_note("0.0", ["0.5", 0, "freq", 143.0])
make_drum("0.25")
make_drum("0.25")

# Should work
bundle.add_content(rows_bundle.build())
client.send(bundle.build())
