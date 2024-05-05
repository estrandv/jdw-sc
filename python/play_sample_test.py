import time
from pythonosc import udp_client
import os

wav_file = os.path.dirname(os.path.realpath(__file__)) + "/wav/snare.wav"

# Hardcoded default port of jdw-sc main application
client = udp_client.SimpleUDPClient("127.0.0.1", 13331) # Straight to main application

# Ensure at least one sample exists
client.send_message("/load_sample", [wav_file, "testsamples", 100, "bd"])

def play(index, category, args):
    client.send_message("/play_sample", [
        "test_sample_id", # External id for n_set reference
        "testsamples", # Sample pack to use - see load call above!
        index, # Index in pack or category
        category, # Category - blank equals none
        70 # Execution delay ms
    ] + args)

step = 0.4
short = step / 2

play(0, "bd", ["amp", 2.5])
time.sleep(step)
play(0, "sn", ["ofs", 0.13, "amp", 1.5])
time.sleep(step)
play(0, "bd", ["amp", 2.5])
time.sleep(short)
play(0, "bd", ["amp", 0.8, "ofs", 0.05])
time.sleep(short)
play(0, "to", ["amp", 2.5])
play(0, "sn", ["ofs", 0.13])
time.sleep(short)
play(0, "sn", ["ofs", 0.13])
time.sleep(short / 2)
play(0, "sn", ["ofs", 0.13])
time.sleep(short / 2)
play(0, "cy", ["ofs", 0.03, "sus", 2.0, "amp", 0.3])
