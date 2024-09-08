# Test sending a bundle of packets to be processed one-by-one

from pythonosc import udp_client
from pythonosc import osc_bundle_builder
from pythonosc import osc_message_builder
import time

# Hardcoded default port of jdw-sc main application
client = udp_client.SimpleUDPClient("127.0.0.1", 13331) # Straight to main application

# Create a synthdef to use
with open("synths/example.scd", "r") as synthdef:
    client.send_message("/create_synthdef", synthdef.read())

# Load a sample
import os
wav_file = os.path.dirname(os.path.realpath(__file__)) + "/wav/snare.wav"
client.send_message("/load_sample", [wav_file, "testsamples", 100, "bd", 47])

bundle = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)

def add_msg(addr, args):
    msg = osc_message_builder.OscMessageBuilder(address=addr)
    for arg in args:
        msg.add_arg(arg)
    bundle.add_content(msg.build())

add_msg("/bundle_info", ["batch-send"])

for i in range(1, 2):
    add_msg("/note_on_timed", [
        "example",
        "brute_TEST_HOLD_" + str(i),
        "0.4", # gate time
        0,
        "freq",
        195.0 + (195.0 * (i)),
        "relT",
        0.2 + (i * 1.2)
    ])

add_msg("/play_sample", ["example_id_lol", "testsamples", 47, "", 0, "ofs", 0.0])

# Should work
client.send(bundle.build())
