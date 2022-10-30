pub fn get_server_properties(runner_index: u32, seed: &String) -> String {
    let server_port: u32 = 25565 + runner_index;
    format!(
        "#Minecraft server properties
#Generated by SpicyGarden
enable-jmx-monitoring=false
rcon.port=
gamemode=survival
enable-command-block=false
enable-query=false
level-name=world
motd=SpicyGarden
query.port={}
pvp=true
difficulty=easy
network-compression-threshold=256
require-resource-pack=false
max-tick-time=60000
use-native-transport=true
max-players=2
online-mode=false
enable-status=true
allow-flight=true
broadcast-rcon-to-ops=true
view-distance=4
server-ip=
resource-pack-prompt=
allow-nether=false
server-port={}
enable-rcon=false
sync-chunk-writes=true
op-permission-level=4
prevent-proxy-connections=false
hide-online-players=false
resource-pack=
entity-broadcast-range-percentage=100
simulation-distance=4
rcon.password=
player-idle-timeout=0
debug=false
force-gamemode=false
rate-limit=0
hardcore=false
white-list=false
broadcast-console-to-ops=true
spawn-npcs=false
spawn-animals=false
snooper-enabled=false
function-permission-level=2
text-filtering-config=
spawn-monsters=false
enforce-whitelist=false
resource-pack-sha1=
spawn-protection=4
max-world-size=8000
level-seed={}
",
        server_port, server_port, seed
    )
    .to_string()
}