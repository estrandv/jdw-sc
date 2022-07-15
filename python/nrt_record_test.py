
from pythonosc import udp_client
from pythonosc import osc_bundle_builder
from pythonosc import osc_message_builder
import time

# Hardcoded default port of jdw-sc main application

client = udp_client.SimpleUDPClient("127.0.0.1", 13331) # Straight to main application

msg = osc_message_builder.OscMessageBuilder(address="/bundle_info")
msg.add_arg("nrt_record")
bundle = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)
bundle.add_content(msg.build())
msg2 = osc_message_builder.OscMessageBuilder(address="/nrt_record_info")
msg2.add_arg(120.0)
msg2.add_arg("myfile.wav")
bundle.add_content(msg2.build())

rows_bundle = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)

def add_msg(addr, args):
    bun = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)
    top_msg = osc_message_builder.OscMessageBuilder(address="/bundle_info")
    top_msg.add_arg("timed_msg")
    info_msg = osc_message_builder.OscMessageBuilder(address="/timed_msg_info")
    info_msg.add_arg(0.0)
    note_msg = osc_message_builder.OscMessageBuilder(address=addr)
    for arg in args:
        note_msg.add_arg(arg)
    bun.add_content(top_msg.build())
    bun.add_content(info_msg.build())
    bun.add_content(note_msg.build())
    rows_bundle.add_content(bun.build())

add_msg("/note_on_timed", ["gentle", "gentle_nrt_id", 2.0])

# Should work
bundle.add_content(rows_bundle.build())
client.send(bundle.build())
