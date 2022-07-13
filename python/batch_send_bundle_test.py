# Test sending a bundle of packets to be processed one-by-one

from pythonosc import udp_client
from pythonosc import osc_bundle_builder
from pythonosc import osc_message_builder
import time

# Hardcoded default port of jdw-sc main application
client = udp_client.SimpleUDPClient("127.0.0.1", 13331) # Straight to main application

bundle = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)

def add_msg(addr, args):
    msg = osc_message_builder.OscMessageBuilder(address=addr)
    for arg in args:
        msg.add_arg(arg)
    bundle.add_content(msg.build())

add_msg("/bundle_info", ["batch_send"])

for i in range(1, 3):
    add_msg("/note_on_timed", [
        "miniBrute",
        "miniBrute_TEST_HOLD_" + str(i),
        0.2 + (0.2 * i), # gate time
        "freq",
        195.0 + (4.4 * i)
    ])

add_msg("/play_sample", ["example", 1, ""])

# Should work
client.send(bundle.build())
