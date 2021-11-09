const wiki = require('wikijs').default;

const w = wiki()
w.random(500).then(async (results) => {
  const pages = await Promise.all(results.map(async (title) => {
    const page = await w.page(title)
    return {
      id: title,
      content: await page.rawContent(),
    }
  }));
  console.log(JSON.stringify(pages));
})
