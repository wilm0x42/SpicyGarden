package spicyrice.SpicyGarden;

import java.io.FileWriter;
import java.io.IOException;
import java.util.ArrayList;
import java.util.HashSet;

import org.bukkit.plugin.java.JavaPlugin;
import org.bukkit.Location;
import org.bukkit.StructureType;
import org.bukkit.Bukkit;
import org.bukkit.block.Block;
import org.bukkit.block.Biome;
import org.bukkit.World;

//import org.bukkit.World;
//import org.bukkit.entity.Player;
//import org.bukkit.event.*;
//import org.bukkit.event.player.*;

public class SpicyGarden extends JavaPlugin {
    // distance in all directions to search from spawn, as a radius
    final int spawnSearchDistance = 1000;
    // power to raise the fib spiral to, to change density of the spiral
    final float rescalePower = 1.5f;
    // The turning angle
    final float turnFraction = ((float) Math.sqrt(5.0) - 1.0f) / 2.0f;
    // list of (x, y) pairs already checked
    HashSet<String> checked = new HashSet<String>();
    // number of potential points to sample (not all will necessarily be valid,
    // since some might be duplicates, depending on rescalePower)
    int pointCount = 1600;
    // constant y-value for block queries
    int searchHeight = 64;

    @Override
    public void onEnable() {
        // getServer().getPluginManager().registerEvents(new SpicyListener(), this);

        try {
            Bukkit.getLogger().info("SPICY GARDEN -- LET'S DO THIS");
            World world = org.bukkit.Bukkit.getWorld("world");

            FileWriter dataOutput = new FileWriter("SpicyGardenData.txt");

            dataOutput.write(String.format("Seed: %d\n", world.getSeed()));

            StructureType[] interestingStructures = {
                    StructureType.DESERT_PYRAMID,
                    StructureType.JUNGLE_PYRAMID,
                    StructureType.OCEAN_MONUMENT,
                    StructureType.VILLAGE,
                    StructureType.WOODLAND_MANSION,
            };

            Bukkit.getLogger().info("Yoinking Structures...");

            for (StructureType structure : interestingStructures) {
                Location nearest = world.locateNearestStructure(new Location(world, 0.0, searchHeight, 0.0),
                        structure,
                        spawnSearchDistance,
                        false);
                if (nearest != null) {
                    dataOutput.write(String.format("Structure: (%d,%d,%d) %s\n",
                            nearest.getBlockX(),
                            nearest.getBlockY(),
                            nearest.getBlockZ(),
                            structure.getName()));
                } else {
                    dataOutput.write(String.format("Structure: NOTFOUND %s\n", structure.toString()));
                }
            }
            float multiplier = 2.0f * (float) Math.PI * turnFraction;
            Bukkit.getLogger().info("Yoinking Biomes...");
            for (int i = 0; i < pointCount; i++) {
                float radius = (float) Math.pow(i / (float) pointCount, rescalePower);
                float angle = i * multiplier;
                int x = Math.round((float) Math.cos(angle) * radius * spawnSearchDistance);
                int z = Math.round((float) Math.sin(angle) * radius * spawnSearchDistance);
                String coords = String.format("%d,%d", x, z);
                if (checked.contains(coords)) {
                    continue;
                }
                checked.add(coords);
                // Block block = world.getHighestBlockAt(x, z);
                Block block = world.getBlockAt(x, searchHeight, z);

                Biome biome = block.getBiome();
                int y = block.getY();

                dataOutput.write(String.format("Biome: (%d,%d,%d) %s\n", x, y, z, biome.toString()));
            }
            // Bukkit.getLogger().info("Point Count: " + pointCount);
            // Bukkit.getLogger().info("Missed: " + (pointCount - checked.size()));
            Bukkit.getLogger().info("Finishing up...");

            dataOutput.close();
        } catch (IOException e) {
        }

        Bukkit.getLogger().info("Shutting down server...");
        Bukkit.shutdown();
    }

    @Override
    public void onDisable() {
    }
}
