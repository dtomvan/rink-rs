import init, * as Rink from "rink-js";
import type { ExecuteRes, HelloRes, RinkRequest } from "./proto";

let rink: Rink.InitOutput | null = null;
let ctx: Rink.Context | null = null;
onmessage = (event: MessageEvent<RinkRequest>) => {
	const msg = event.data;
	console.log("worker recv: ", msg);
	if (msg.type == "hello") {
		init(msg.buffer).then((res) => {
			rink = res;
			ctx = new Rink.Context();
			const response: HelloRes = {
				type: "hello",
				version: Rink.version(),
			};
			console.log("worker send: ", response);
			postMessage(response);
		});
	} else if (msg.type == "execute") {
		let response: ExecuteRes;
		if (!ctx) {
			response = {
				type: "execute",
				status: "error",
				id: msg.id,
				message: "execute message sent before hello message"
			};
		} else {
			const query = new Rink.Query(msg.query);
			const tokens = ctx.eval_tokens(query);

			response = {
				type: "execute",
				status: "success",
				id: msg.id,
				tokens,
			};
		}
		console.log("worker send: ", response);
		postMessage(response);
	}
}