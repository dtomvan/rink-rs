import init, * as rink from 'rink-js';

// Taken from https://stackoverflow.com/a/3809435
const urlRegex = /https?:\/\/(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)/g;
const powRegex = /\^(\-?\d+)/g;

type TokenType = "plain" | "error" | "unit" | "quantity" | "number" | "user_input" | "list_begin" | "list_sep" | "doc_string" | "pow" | "prop_name" | "date_time"

type Token = {
	type: "span",
	text: string,
	fmt: TokenType,
};
type TokenList = {
	type: "list",
	children: [SpanOrList],
}
type SpanOrList = Token | TokenList;

function buildInline(parent: HTMLElement, text: string) {
	// escape any html tags
	parent.innerText = text;
	text = parent.innerHTML;
	// apply the regexes
	text = text.replace(urlRegex, (match) =>
		`<a href="${match}" rel="nofollow">${match}</a>`
	);
	text = text.replace(powRegex, (_match, rest) =>
		`<sup>${rest}</sup>`
	);
	parent.innerHTML = text;
}

const dateFmt = new Intl.DateTimeFormat(undefined, {
	dateStyle: "long",
	timeStyle: "long",
})

function buildHtml(tokens: [SpanOrList], parent: HTMLElement) {
	let ul: HTMLUListElement | null = null;
	let cur: HTMLElement = parent;
	for (const token of tokens) {
		if (token.type == "list") {
			buildHtml(token.children, cur);
		} else if (token.fmt == "pow") {
			let text = token.text.replace(/^\^/, '');
			let sup = document.createElement("sup");
			sup.innerText = text;
			cur.appendChild(sup);
		} else if (token.fmt == "list_begin") {
			ul = document.createElement("ul");
			parent.appendChild(ul);
			let li = document.createElement("li");
			cur = li;
			ul.appendChild(li);
		} else if (token.fmt == "list_sep" && ul) {
			let li = document.createElement("li");
			cur = li;
			ul.appendChild(li);
		} else if (token.fmt == "date_time") {
			let time = document.createElement("time");
			let date = new Date(token.text);
			time.setAttribute("datetime", date.toISOString());
			time.innerText = dateFmt.format(date);
			cur.appendChild(time);
		} else {
			let span = document.createElement("span");
			span.classList.add(`hl-${token.fmt.replace('_', '-')}`);
			buildInline(span, token.text);
			cur.appendChild(span);
		}
	}
}

init().then(() => {
	console.log('hello', rink);

	let rinkDiv: HTMLElement = document.querySelector("#rink-outputs")!;
	let form: HTMLFormElement = document.querySelector("#rink-query")!;
	let textEntry: HTMLInputElement = document.querySelector("#query")!;

	let ctx = new rink.Context();
	ctx.setSavePreviousResult(true);
	console.log(ctx);

	let welcome = document.createElement("p");
	welcome.innerText = `Rink ${rink.version()}`;
	rinkDiv.innerHTML = '';
	rinkDiv.appendChild(welcome);

	form.addEventListener("submit", (event) => {
		event.preventDefault();

		let quote = document.createElement("blockquote");
		quote.innerText = textEntry.value;
		rinkDiv.appendChild(quote);

		let query = new rink.Query(textEntry.value);
		ctx.setTime(new Date());
		let tokens = ctx.eval_tokens(query);
		console.log(tokens);

		let p = document.createElement("p");
		buildHtml(tokens, p);

		rinkDiv.appendChild(p);
		textEntry.value = "";
		window.scrollTo(0, document.body.scrollHeight);
	});
});
