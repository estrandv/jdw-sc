
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

bad_msg = osc_message_builder.OscMessageBuilder(address="/bundle_info")
bad_msg.add_arg(0.0)
bad_bundle = osc_bundle_builder.OscBundleBuilder(osc_bundle_builder.IMMEDIATELY)
bad_bundle.add_content(bad_msg.build())

# Should fail
client.send(bad_bundle.build())

# Should work
client.send(bundle.build())
