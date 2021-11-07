const MiniSearch = require('minisearch')
const fs = require('fs')

const configPath = process.argv[2]
const sourcePath = process.argv[3]
const times = parseInt(process.argv[4]) || 0

const config = JSON.parse(fs.readFileSync(configPath))

const source = JSON.parse(fs.readFileSync(sourcePath))
if (times) {
    for (let i = 0; i < times; i++) {
        const ms = new MiniSearch(config)
        ms.addAll(source)
        JSON.stringify(ms.toJSON())
    }
} else {
    const ms = new MiniSearch(config)
    ms.addAll(source)
    console.log(JSON.stringify(ms.toJSON()))
}
