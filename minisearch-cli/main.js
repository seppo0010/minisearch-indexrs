const MiniSearch = require('minisearch')
const fs = require('fs')

const configPath = process.argv[2]
const sourcePath = process.argv[3]

const config = JSON.parse(fs.readFileSync(configPath))

const source = JSON.parse(fs.readFileSync(sourcePath))
const ms = new MiniSearch(config)
ms.addAll(source)
console.log(JSON.stringify(ms.toJSON()))
