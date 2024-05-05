from pythonosc import udp_client
import time

client = udp_client.SimpleUDPClient("127.0.0.1", 13331) # Straight to main application

# Or whatever your path is on this particular day ....
client.send_message("/load_sample", ["/home/estrandv/sample_packs/GBA/GBA-SP Perc3.wav", "testsamples", 100, "bd"])

time.sleep(0.5)

client.send_message("/play_sample", [
    "test_sample_id", # External id for n_set reference
    "testsamples", # Sample pack to use - a dir-name in "sample_packs"
    0, # Index in pack or category
    "bd", # Category - blank equals none
    0 # Execution delay ms
] + ["amp", 1.0])
