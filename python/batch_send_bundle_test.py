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

for i in range(1, 8):
    add_msg("/note_on_timed", [
        "miniBrute",
        "miniBrute_TEST_HOLD_" + str(i),
        0.2 + (0.1 * i),
        "freq",
        235.0 + (44.0 * i),
        "attT",
        0.7 * i,
        "relT",
        i * 0.5,
        "lfoS",
        4.0 + (i * 800.0),
        "lfoD",
        0.2 + (i * 0.02)
    ])

add_msg("/play_sample", ["example", 0, ""])

# Should work
client.send(bundle.build())
