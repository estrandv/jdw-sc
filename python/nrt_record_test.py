
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
msg2.add_arg(4.0) # A smarter program would adjust this according to timed messages added (end beat)
bundle.add_content(msg2.build())

rows_bundle = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)

def add_msg(time, addr, args):
    args = ["brute", "gentle_nrt_id"] + args
    bun = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)
    top_msg = osc_message_builder.OscMessageBuilder(address="/bundle_info")
    top_msg.add_arg("timed_msg")
    info_msg = osc_message_builder.OscMessageBuilder(address="/timed_msg_info")
    info_msg.add_arg(time)
    note_msg = osc_message_builder.OscMessageBuilder(address=addr)
    for arg in args:
        note_msg.add_arg(arg)
    bun.add_content(top_msg.build())
    bun.add_content(info_msg.build())
    bun.add_content(note_msg.build())
    rows_bundle.add_content(bun.build())

add_msg(0.0, "/note_on_timed", [0.1])
add_msg(0.5, "/note_on_timed", [1.1])
add_msg(2.0, "/note_on_timed", [0.05])

# Should work
bundle.add_content(rows_bundle.build())
client.send(bundle.build())
