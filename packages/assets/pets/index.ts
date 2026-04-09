import type { PetSpecies } from '@octopus/schema'

import axolotlAsset from './axolotl.svg'
import blobAsset from './blob.svg'
import cactusAsset from './cactus.svg'
import capybaraAsset from './capybara.svg'
import catAsset from './cat.svg'
import chonkAsset from './chonk.svg'
import dragonAsset from './dragon.svg'
import duckAsset from './duck.svg'
import ghostAsset from './ghost.svg'
import gooseAsset from './goose.svg'
import mushroomAsset from './mushroom.svg'
import octopusAsset from './octopus.svg'
import owlAsset from './owl.svg'
import penguinAsset from './penguin.svg'
import rabbitAsset from './rabbit.svg'
import robotAsset from './robot.svg'
import snailAsset from './snail.svg'
import turtleAsset from './turtle.svg'

export const petAssetMap = {
  axolotl: axolotlAsset,
  blob: blobAsset,
  cactus: cactusAsset,
  capybara: capybaraAsset,
  cat: catAsset,
  chonk: chonkAsset,
  dragon: dragonAsset,
  duck: duckAsset,
  ghost: ghostAsset,
  goose: gooseAsset,
  mushroom: mushroomAsset,
  octopus: octopusAsset,
  owl: owlAsset,
  penguin: penguinAsset,
  rabbit: rabbitAsset,
  robot: robotAsset,
  snail: snailAsset,
  turtle: turtleAsset,
} satisfies Record<PetSpecies, string>
