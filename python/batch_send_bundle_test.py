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

add_msg("/bundle_info", ["batch-send"])

for i in range(1, 2):
    add_msg("/note_on_timed", [
        "brute",
        "brute_TEST_HOLD_" + str(i),
        "0.4", # gate time
        0,
        "freq",
        195.0 + (195.0 * (i)),
        "relT",
        0.2 + (i * 1.2)
    ])

add_msg("/play_sample", ["example_id_lol", "example", 47, "", 0])

# Should work
client.send(bundle.build())
