# Civ Map Generator

This is a civilization map generator. This algorithm is primarily based on the implementation in *Civilization V*, with some references from *Civilization VI*.

## Innovation Highlights

This project introduces several key innovations:

1. **Support both flat and pointy hex**  
   Original civilization implementation only supports pointy hex, Unciv implementation only supports flat hex, but this project supports both flat and pointy hex.


## Miss Features

1. **No support to add feature atoll**  
   This project has not implemented the feature of adding atoll to the map.
2. **Only support to generate fractal and pangaea map**  
   This project only supports to generate fractal and pangaea map. we will add more map generation algorithm in the future.
3. **No support to square grid**
   This project only supports hex grid. We will add support to square grid in the future.
4. **The algorithm to add rivers is not perfect**
   The algorithm to add rivers is not perfect. We should tackle with the situation when river flows to the edge of map.


## Reference project

[Unciv](https://github.com/yairm210/Unciv)