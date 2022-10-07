package spicyrice.SpicyGarden;

import java.io.FileWriter;
import java.io.IOException;

import org.bukkit.plugin.java.JavaPlugin;
import org.bukkit.Location;
import org.bukkit.StructureType;
import org.bukkit.Bukkit;
//import org.bukkit.World;
//import org.bukkit.entity.Player;
//import org.bukkit.event.*;
//import org.bukkit.event.player.*;

public class SpicyGarden extends JavaPlugin {
    int spawn_search_distance = 1000; // distance in all directions to search from spawn, in a square
    int search_point_spacing = 50; // distance between blocks to query for data
    int search_height = 64; // constant y-value for block queries

    @Override
    public void onEnable() {
        // getServer().getPluginManager().registerEvents(new SpicyListener(), this);

        try {
            Bukkit.getLogger().info("SPICY GARDEN -- LET'S DO THIS");
            org.bukkit.World world = org.bukkit.Bukkit.getWorld("world");

            FileWriter data_output = new FileWriter("SpicyGardenData.txt");

            data_output.write(String.format("Seed: %d\n", world.getSeed()));

            StructureType[] interesting_structures = {
                    StructureType.DESERT_PYRAMID,
                    StructureType.JUNGLE_PYRAMID,
                    StructureType.OCEAN_MONUMENT,
                    StructureType.VILLAGE,
                    StructureType.WOODLAND_MANSION,
            };

            Bukkit.getLogger().info("Yoinking Structures...");

            for (StructureType structure : interesting_structures) {
                Location nearest = world.locateNearestStructure(new Location(world, 0.0, search_height, 0.0),
                        structure,
                        spawn_search_distance,
                        false);
                if (nearest != null) {
                    data_output.write(String.format("Structure: (%d,%d,%d) %s\n",
                            nearest.getBlockX(),
                            nearest.getBlockY(),
                            nearest.getBlockZ(),
                            structure.getName()));
                } else {
                    data_output.write(String.format("Structure: NOTFOUND %s\n", structure.toString()));
                }
            }

            Bukkit.getLogger().info("Yoinking Biomes...");

            for (int x = -spawn_search_distance; x < spawn_search_distance; x += search_point_spacing) {
                for (int z = -spawn_search_distance; z < spawn_search_distance; z += search_point_spacing) {
                    org.bukkit.block.Block block = world.getBlockAt(x, search_height, z);

                    org.bukkit.block.Biome biome = block.getBiome();

                    data_output.write(String.format("Biome: (%d,%d,%d) %s\n", x, search_height, z, biome.toString()));
                }
            }

            Bukkit.getLogger().info("Finishing up...");

            data_output.close();
        } catch (IOException e) {
        }

        Bukkit.getLogger().info("Shutting down server...");
        Bukkit.shutdown();
    }

    @Override
    public void onDisable() {
    }
}
