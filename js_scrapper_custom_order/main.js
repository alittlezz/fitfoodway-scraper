// filter/sort however you need
let id_to_info = await get_id_to_info(10);

let menu = new Map([
    ["Monday", [28, 39, 16, 24]],
    ["Tuesday", [28, 39, 16]],
    ["Wednesday", [28, 39, 16, 24]],
    ["Thursday", [28, 39, 16]],
    ["Friday", [28, 39, 16, 24]],
]);

await calculate_macro_per_day(id_to_info, menu);

let days = 5;
let starting_date = new Date("2023-02-06");

// await fill_basket(menu, days, starting_date);

async function calculate_macro_per_day(id_to_info, menu) {
    for (let [day, products] of menu) {
        let day_info = {
            menu: [],
            price: 0,
            macro: {
                grams: 0,
                kcal: 0,
                carbohydrates: 0,
                fats: 0,
                proteins: 0,
                fibers: 0,
            }
        };
        for (let product_id of products) {
            const product = id_to_info.get(product_id.toString());

            day_info.menu.push(product.name);
            day_info.price += product.price;
            day_info.macro.grams += product.macro.grams;
            day_info.macro.kcal += product.macro.kcal;
            day_info.macro.carbohydrates += product.macro.carbohydrates;
            day_info.macro.fats += product.macro.fats;
            day_info.macro.proteins += product.macro.proteins;
            day_info.macro.fibers += product.macro.fibers;
        }

        console.log(`On ${day} the menu is: ${day_info.menu.join(", ")} - ${day_info.price.toFixed(2)} Lei: \n`
            + `${JSON.stringify(day_info.macro)}.`);
    }
}

async function get_id_to_info(off = 0) {
    console.log("Getting information from all products.");
    const name_to_id = await get_name_to_id();

    let id_to_info = new Map();
    await Promise.all(Array.from(name_to_id).map(([name, id]) => get_info(name, id)));

    for (let [id, info] of id_to_info) {
        console.log(`${id} - ${info.name}: ${info.price.toFixed(2)} Lei => ${JSON.stringify(info.macro)}`);
    }
    return id_to_info;

    async function get_info(name, id) {
        const product_page = await fetch(`https://fitfoodway.ro/p/${name}`).then(r => r.text());
        const doc = new DOMParser().parseFromString(product_page, "text/html");

        const value_regex = /\s*(\d+(:?[.,]\d+)?)/;
        const lines = Array.from(doc.querySelectorAll(".price, div.amount-per-serving > div"))
            .map(l => l.innerText)
            .flatMap(l => l.split(/\r?\n/))
            .map(l => l.match(value_regex))
            .filter(m => m)
            .map(m => m[1].replace(',', '.'))
            .map(v => parseFloat(v));

        id_to_info.set(id, {
            name: doc.querySelector(".banner-text h1").innerText,
            price: lines[0] * (100 - off) / 100,
            macro: {
                grams: lines[1],
                kcal: lines[2],
                carbohydrates: lines[3],
                fats: lines[4],
                proteins: lines[5],
                fibers: lines[6],
            }
        });
    }

    async function get_name_to_id() {
        let main_page = await fetch("https://fitfoodway.ro/produse").then(r => r.text());
        let doc = new DOMParser().parseFromString(main_page, "text/html");

        let name_to_id = new Map();
        let products = doc.querySelectorAll(".menu-item-wrap > div.content");
        for (let prod of products) {
            let name_regex = /.*\/(.*)$/;
            let name = prod.querySelector("h2 > a").href.match(name_regex)[1];

            let id_regex = /adauga_in_cos\((\d+),/;
            let id = prod.querySelector("a.btn").attributes.onclick.nodeValue.match(id_regex)[1];

            name_to_id.set(name, id);
        }
        return name_to_id;
    }
}

async function fill_basket(menu, days, date = new Date()) {
    const weekday = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"];
    const product_count = weekday.map(w => menu.get(w)?.length).filter(l => !isNaN(l)).reduce((a, b) => a + b, 0);
    if (product_count < 1) {
        console.log("Empty product list, nothing to add.")
        return;
    }
    console.log("Filling basket.");
    while (days > 0) {
        let to_add = menu.get(weekday[date.getDay() - 1]);
        if (to_add) {
            await Promise.all(to_add.map(id => add_product(date, id)));
            console.log(`Added ${to_add} on ${date.toISOString().split('T')[0]}.`);
            days--;
        }
        date.setDate(date.getDate() + 1);
    }
    console.log("Basket filled.");
    console.log("Adding off code.");
    await add_off_code();
}

async function add_product(date, id, type = "produs") {
    date = date.toISOString().split('T')[0];
    await post("https://fitfoodway.ro/comanda/adauga_in_cos",
        `tip_id=${id}&tip=${type}&date=${date}`,
        `Failed sending request for type:${type}, id:${id}, date:${date}.`
    );
}

async function add_off_code(code = "WELCOME") {
    await post("https://fitfoodway.ro/cos/adauga_cod",
        `cod_reducere=${code}`,
        `Failed applying off code "${code}".`
    );
}

async function post(url, body, err_message) {
    return await fetch(url, {
        "headers": {
            "content-type": "application/x-www-form-urlencoded",
        },
        "body": body,
        "method": "POST",
    }).catch(err => {
        console.log(err_message, err);
    });
}