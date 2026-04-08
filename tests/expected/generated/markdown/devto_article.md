Native voice input and idle task APIs

I love writing philosophical essays — thoughts about code, work, all that stuff. I also love deep technical dives. But I know *you* love my lists of cool features that not everyone has heard about yet 😄

What’s up with me? This week I’m preparing for a conference, fighting performance issues, and trying to get at least somewhat ready for the upcoming holidays 😉

Something nice happened too. I enjoy writing — not just technical articles, but in general. Last summer my life changed quite a bit, and to keep my sanity I started writing a sci-fi story, which I submitted to a Polish science fiction foundation competition. I didn’t win, but my story made it pretty far — around 13th place out of 179 submissions. Considering it was my first attempt at this kind of writing… it could have gone worse 😄

And speaking of sci-fi — the kind happening right in front of us 😉 Today I’ve prepared a batch of things the browser can already do, which honestly didn’t fit in my head not that long ago. A lot of these are still not that widely known, and yet many of them are already supported across modern browsers. Have fun!

---

## [#1-let-me-just-run-this-later-%E2%86%92-raw-requestidlecallback-endraw-](#1-let-me-just-run-this-later-%E2%86%92-raw-requestidlecallback-endraw-) 1. “Let me just run this later” → `requestIdleCallback`

At first I thought this API was pointless. It basically lets you run some code when nothing interesting is happening. Ok… cool… but why would I care?

Turns out — there are tons of use cases. For example, collecting data about how the user behaves on your page — definitely not something you want to do while your 200 components are rendering 😅 Or loading less important data, preprocessing something, generating images in the background.

Honestly, there are probably as many use cases as there are developers.

```
function trackUserScrolling() {
  console.log("User scrolled. This changes everything.");
}
if ("requestIdleCallback" in window) {
  requestIdleCallback(trackUserScrolling);
} else {
  setTimeout(trackUserScrolling, 0);
}
```

**Support:** modern browsers (historically missing in Safari, so fallback is still a good idea)

---

## [#2-why-is-my-input-not-highlighting-%E2%86%92-raw-focuswithin-endraw-](#2-why-is-my-input-not-highlighting-%E2%86%92-raw-focuswithin-endraw-) 2. “Why is my input not highlighting???” → `:focus-within`

It’s easy to style an element that has focus. But what if you want to style the parent div? For example, make it pink, add some flowers 😉 You can write 40 lines of JavaScript… or just use `:focus-within`.

Works. No listeners. No bugs. No suffering.

```
.form-field {
  border: 1px solid #ccc;
  padding: 12px;
}
.form-field:focus-within {
  border-color: hotpink;
}
```

```
&lt;div class="form-field"&gt;
  &lt;input placeholder="Type something meaningful..." /&gt;
&lt;/div&gt;
```

**Support:** basically everywhere that matters

---

## [#3-lets-show-offline-mode-%E2%86%92-raw-navigatoronline-endraw-](#3-lets-show-offline-mode-%E2%86%92-raw-navigatoronline-endraw-) 3. “Let’s show offline mode” → `navigator.onLine`

Have you ever built a PWA? Because I have, and the eternal problem is what to do when the user loses connection (e.g. they’re in the wilderness or just walked into an elevator 😄). You can write a bunch of complicated ifs, or just listen to `offline` and `online`. On `offline` you can store data in IndexedDB, and when the user is back online, send it to the server.

```
window.addEventListener("offline", () =&gt; {
  alert("You are offline. Time to panic.");
});
window.addEventListener("online", () =&gt; {
  alert("You're back. Panic cancelled.");
});
```

**Support:** widely supported (but “online” ≠ “your backend works” 😅)

---

## [#4-smooth-animation-but-make-it-cursed-%E2%86%92-raw-requestanimationframe-endraw-](#4-smooth-animation-but-make-it-cursed-%E2%86%92-raw-requestanimationframe-endraw-) 4. “Smooth animation, but make it cursed” → `requestAnimationFrame`

We’ve all seen this:

```
setInterval(() =&gt; {
  element.style.left = Math.random() * 100 + "px";
}, 16);
```

You can *feel* this is not the best idea 😉 It just lags. Luckily we have `requestAnimationFrame`, which is synced with the browser repaint cycle, so things are actually smooth.

```
function animate() {
  element.style.transform = `translateX(${Date.now() % 300}px)`;
  requestAnimationFrame(animate);
}
requestAnimationFrame(animate);
```

**Support:** everywhere

---

## [#5-this-card-should-adapt-but-only-here-%E2%86%92-container-queries](#5-this-card-should-adapt-but-only-here-%E2%86%92-container-queries) 5. “This card should adapt… but only here” → container queries

This feature feels almost unfair. I’m at a point in my career where I barely write CSS anymore (well, except for occasional moments like the one I described here: [Is learning CSS a waste of time in 2026?](https://dev.to/sylwia-lask/is-learning-css-a-waste-of-time-in-2026-nj3)).

But there was a time when I wrote *a lot* of it. And wow — how much I would have given to apply media queries to a specific element instead of the whole viewport. Now we finally can. The component becomes self-aware, and we can go grab a coffee.

```
.card-wrapper {
  container-type: inline-size;
}
.card {
  display: grid;
}
@container (min-width: 400px) {
  .card {
    grid-template-columns: 1fr 2fr;
  }
}
```

**Support:** modern browsers (add fallback if needed)

---

## [#6-random-id-what-could-go-wrong-%E2%86%92-raw-cryptogetrandomvalues-endraw-](#6-random-id-what-could-go-wrong-%E2%86%92-raw-cryptogetrandomvalues-endraw-) 6. “Random ID, what could go wrong?” → `crypto.getRandomValues`

```
const id = Math.random().toString(36).slice(2);
```

This is how bugs are born. It looks like “good enough” crypto from AliExpress and works… until it doesn’t. First of all, it depends on the engine implementation — we don’t really know what’s happening under the hood. Some patterns are absolutely possible, and with enough IDs you’re basically asking for duplicates.

Luckily, we now have a simple native solution. It’s not a silver bullet, but `crypto.getRandomValues` is pretty solid — much better entropy, no weird patterns, dramatically reduces the chance of collisions. The browser just does it properly.

```
const bytes = new Uint8Array(8);
crypto.getRandomValues(bytes);
const id = Array.from(bytes)
  .map(b =&gt; b.toString(16).padStart(2, "0"))
  .join("");
console.log("Secure-ish ID:", id);
```

**Support:** widely supported

---

## [#7-we-need-a-modal-%E2%86%92-raw-ltdialoggt-endraw-](#7-we-need-a-modal-%E2%86%92-raw-ltdialoggt-endraw-) 7. “We need a modal” → `&lt;dialog&gt;`

It’s honestly nice that browsers finally stepped up and said: fine, here’s your modal 😄 No more installing 12KB libraries just to open a dialog that users love so much. This one is also accessible by default, so win-win.

```
&lt;dialog id="modal"&gt;
  &lt;p&gt;Are you sure you want to deploy on Friday?&lt;/p&gt;
  &lt;button onclick="modal.close()"&gt;Cancel&lt;/button&gt;
  &lt;button onclick="alert('Good luck 😬')"&gt;Deploy&lt;/button&gt;
&lt;/dialog&gt;
&lt;button onclick="modal.showModal()"&gt;Open modal&lt;/button&gt;
```

**Support:** modern browsers

---

## [#8-voice-input-would-be-cool-%E2%86%92-speech-api](#8-voice-input-would-be-cool-%E2%86%92-speech-api) 8. “Voice input would be cool…” → Speech API

Are you already installing transformers.js because you need speech recognition? Relax — turns out the browser has something for that too. Well… at least Chromium does 😄 So if you can “encourage” users to use Chrome, Edge, or something similar, you’re good. Personally, I’d still be careful with production use, but for demos? Why not.

```
const SpeechRecognition =
  window.SpeechRecognition || window.webkitSpeechRecognition;
if (SpeechRecognition) {
  const recognition = new SpeechRecognition();
  recognition.onresult = e =&gt; {
    console.log("You said:", e.results[0][0].transcript);
  };
  recognition.start();
}
```

**Support:** mostly Chromium

---

## [#9-will-this-css-explode-%E2%86%92-raw-supports-endraw-](#9-will-this-css-explode-%E2%86%92-raw-supports-endraw-) 9. “Will this CSS explode?” → `@supports`

Here’s a modern solution to the classic “it works on my machine” — at least in CSS 😉 You don’t have to guess whether something will break your layout. Just wrap it in `@supports`. There is a small catch — while support is very good, it’s not literally everywhere, so ironically… we could use `@supports` for `@supports`.

```
.card {
  background: white;
}
@supports (backdrop-filter: blur(10px)) {
  .card {
    backdrop-filter: blur(10px);
    background: rgba(255, 255, 255, 0.6);
  }
}
```

**Support:** very good

---

## [#but-dont-get-me-wrong](#but-dont-get-me-wrong) ⚠️ But don’t get me wrong

Libraries are great. Sometimes you absolutely need them. But sometimes… you’re installing a dependency for something the browser solved years ago. Before installing anything, just ask yourself (or Google): “Is the browser already smarter than me here?” Sometimes the answer is yes. And that’s… perfectly fine 😄
[[Image: profile] MongoDB](https://dev.to/mongodb) Promoted
- [What's a billboard?](https://dev.to/billboards)
- [Manage preferences](https://dev.to/settings/customization)
- [Report billboard](https://dev.to/report-abuse?billboard=241233)

[[Image: Scale globally with MongoDB Atlas. Try free.]](https://www.mongodb.com/cloud/atlas/lp/try3?amp%3Butm_source=devto&amp%3Butm_medium=display&amp%3Butm_content=scalbglobally-v1&amp%3Bbb=241233)

## [#scale-globally-with-mongodb-atlas-try-free](#scale-globally-with-mongodb-atlas-try-free) [Scale globally with MongoDB Atlas. Try free.](https://www.mongodb.com/cloud/atlas/lp/try3?amp%3Butm_source=devto&amp%3Butm_medium=display&amp%3Butm_content=scalbglobally-v1&amp%3Bbb=241233)

MongoDB Atlas is the global, multi-cloud database for modern apps trusted by developers and enterprises to build, scale, and run cutting-edge applications, with automated scaling, built-in security, and 125+ cloud regions.

[Learn More](https://www.mongodb.com/cloud/atlas/lp/try3?amp%3Butm_source=devto&amp%3Butm_medium=display&amp%3Butm_content=scalbglobally-v1&amp%3Bbb=241233)
Read More &nbsp; [[Image: the_nortern_dev profile image]](https://dev.to/the_nortern_dev) [NorthernDev](https://dev.to/the_nortern_dev) NorthernDev [NorthernDev](https://dev.to/the_nortern_dev) Follow Senior Developer advocating for The Boring Stack. Building Sigilla to fix knowledge management. Writing about the intersection of AI and software craftsmanship. Contact: nordicsecures@proton.me
- Location North Sweden
- Education Computer Science and Systems Design. Focused on building reliable, long-term software.
- Work Senior Freelance Developer &amp; Founder of Sigilla. Focus on pragmatic architecture and backend scale.
- Joined Nov 26, 2025
• [Apr 2](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fthe_nortern_dev%2Fcomment%2F36aka)

Casually mentioning that you almost won a national sci-fi competition right before diving into native browser APIs is an incredible flex. I am genuinely impressed by that combination of skills. 🥳

The technical list is spot on. The amount of heavy dependencies I have seen installed just to replicate that exact native dialog behavior is depressing. Also, your offline mode panic alert logic is exactly the kind of architecture the web needs more of.

Good luck with the conference this week. Try not to let the performance issues keep you up writing code all night. We both know exactly where that leads. Get some actual rest before the holidays, and let me know how the presentation goes. 😃
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 2](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36ap4)

Haha, of course my first reaction was: “well… that’s a failure” because it wasn’t one of the top spots 😄 But after a moment I was like… hmm, maybe that’s actually not bad at all 😄

And thank you! The conference is exactly a week from now, so it’s the final stretch now!
&nbsp; [[Image: the_nortern_dev profile image]](https://dev.to/the_nortern_dev) [NorthernDev](https://dev.to/the_nortern_dev) NorthernDev [NorthernDev](https://dev.to/the_nortern_dev) Follow Senior Developer advocating for The Boring Stack. Building Sigilla to fix knowledge management. Writing about the intersection of AI and software craftsmanship. Contact: nordicsecures@proton.me
- Location North Sweden
- Education Computer Science and Systems Design. Focused on building reliable, long-term software.
- Work Senior Freelance Developer &amp; Founder of Sigilla. Focus on pragmatic architecture and backend scale.
- Joined Nov 26, 2025
• [Apr 2](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fthe_nortern_dev%2Fcomment%2F36b24)

Only a developer could beat out over a hundred and sixty other writers on their very first attempt and instinctively classify it as a failure. That exact brand of relentless perfectionism is probably why your technical work is so solid, but you have to admit it is a completely ridiculous standard to hold yourself to. You should absolutely just own that success. 😄

​Good luck with the final stretch of preparations this week. Just try to resist the urge to rewrite your entire presentation the night before. By the way, is the conference going to be streamed anywhere? Let me know if there is a link. I would really like to tune in and watch you present.😊
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 2](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36bce)

Haha, you’re probably right about that 😄

As for the conference, it most likely won’t be streamed live, but it should be uploaded to YouTube afterward, so I’ll definitely share the link once it’s out! 😊 &nbsp; [[Image: pengeszikra profile image]](https://dev.to/pengeszikra) [Peter Vivo](https://dev.to/pengeszikra) Peter Vivo [[Image: Subscriber]](https://dev.to/++) [Peter Vivo](https://dev.to/pengeszikra) Follow Pipeline operator and touch bar fanatic from Hungary. Vibe Archeologist God speed you!
- Location Pomaz
- Education streetwise
- Work full stack developer at TCS
- Joined Jul 24, 2020
• [Apr 2](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fpengeszikra%2Fcomment%2F36afe)

Wow the times os going ower my head, I don't know about voice input is aviable in browser. (my Mac is know that by douple press fn by default)
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 2](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36agc)

Haha yeah, it’s one of those features that feels a bit like sci-fi creeping into everyday life 😄 You’re right though, the support is still a bit limited and not something you’d rely on everywhere in production yet. But things like what you mentioned on macOS show exactly where this is heading. Feels like we’re slowly moving toward voice being just another normal input method in apps 👀 &nbsp; [[Image: jon_at_backboardio profile image]](https://dev.to/jon_at_backboardio) [Jonathan Murray](https://dev.to/jon_at_backboardio) Jonathan Murray [Jonathan Murray](https://dev.to/jon_at_backboardio) Follow Co-Founder of Backboard.io, your entire AI stack in one API, built on the worlds #1 AI memory infrastructure.
- Location Ottawa, Ontario
- Joined Mar 14, 2026
• [Apr 4](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fjon_at_backboardio%2Fcomment%2F36d5j)

The `requestIdleCallback` example is a good opener — it's one of those APIs that solves a real problem (deferring non-critical work without blocking the main thread) that most developers reach for a setTimeout hack to approximate.

A few others worth adding to the "browser already solved it" list: the Intersection Observer API (replaces hand-rolled scroll listeners for lazy loading and animation triggers), the View Transitions API for page transition animations that most people build with JavaScript frameworks, ResizeObserver for element-level resize detection (no more polling or window resize hacks), and `dialog` element with `showModal()` for accessible modals without a single line of focus-trap JavaScript.

The pattern across all of these is the same: they exist because the browser vendors watched what developers were hacking together repeatedly and standardized the good version. The bottleneck is usually awareness — most developers don't audit their dependencies against the platform periodically. Worth doing annually.
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 4](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36d72)

Exactly this 🙌 I’ve actually written about some of these in my previous articles — things like Intersection Observer or ResizeObserver are such good examples of “the browser already solved it.”

I also really like the idea of doing a yearly audit. That’s honestly something more teams should consider. The problem is, we’re often tied to a specific UI library, and then it’s up to them whether they keep up with modern browser capabilities or not.

And I’ve definitely seen some “genius” cases where datepickers still use Moment.js… even though the Moment docs themselves say not to use it anymore 😄 &nbsp; [[Image: leob profile image]](https://dev.to/leob) [leob](https://dev.to/leob) leob [leob](https://dev.to/leob) Follow
- Pronouns he/him
- Joined Aug 3, 2017
• [Apr 3](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fleob%2Fcomment%2F36bpm)

Cool article, but this is only the tip of the iceberg - the native browser/web APIs have become so extensive, most devs know only a tiny bit of what's available (to a large extent because they use frameworks like React to program the frontend)...

What do you think of the idea (and feasibility) to **not** use React or other frameworks to build a frontend, but only "custom elements" (the preferred contemporary name for "web components", I've learned) and native browser APIs - is it a realistic alternative?
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 3](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36c22)

That’s a great question!

If I had to answer quickly, I’d say: probably not as a full replacement in most cases. Frameworks solve a lot of problems for us, they come with mature ecosystems, component libraries, state management patterns, routing, etc. So they’re not bad at all. The real issue is more that developers tend to overdo it with additional libraries on top of them.

That said… I’ve actually been thinking about this more recently, and I’m not so sure anymore, especially for simpler apps. With custom elements and modern browser APIs, you can go surprisingly far without a framework. So maybe it’s not about replacing frameworks entirely, but being more intentional about when we actually need them.
&nbsp; [[Image: leob profile image]](https://dev.to/leob) [leob](https://dev.to/leob) leob [leob](https://dev.to/leob) Follow
- Pronouns he/him
- Joined Aug 3, 2017
• [Apr 3 • Edited on Apr 3 • Edited](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fleob%2Fcomment%2F36c29)

Great take, excellent - this:

*"Maybe it’s not about replacing frameworks entirely, but being more intentional about when we actually need them"*

That's exactly the gist of this article which I just came across:

[blog.logrocket.com/anti-frameworki...](https://blog.logrocket.com/anti-frameworkism-native-web-apis)

So yeah, native web APIs and 'custom elements' are not the holy grail, neither are frameworks - each has its place...

And this one:

*"The real issue is more that developers tend to overdo it with additional libraries on top of them"*

Yes, we're way too 'easy'/lazy pulling in tons of dependencies even when we don't really need them, also for trivial things - which has drawbacks, **and** risks:

[a16z.news/p/et-tu-agent-did-you-in...](https://www.a16z.news/p/et-tu-agent-did-you-install-the-backdoor)

But, if you rely on AI coding tools/agents - they tend to favor... React:

[dev.to/krunal_groovy/vue-vs-react-...](https://dev.to/krunal_groovy/vue-vs-react-in-2026-what-ai-first-development-teams-actually-choose-419b)

So you need to make a conscious choice, and maybe you need to put in a little bit more effort...
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 3](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36c79)

Wow, that’s a lot of great content, saved! 🙌

As for preferring React, that doesn’t surprise me at all. It’s simply the most popular, so AI was probably trained on it the most. And of course, it’s often a good choice — I like React too, but not always.

At work, for my large enterprise project, we use Angular — and I really appreciate it. It’s stable, a lot of things work out of the box, and I’ve been blissfully avoiding vulnerability dramas for years 😄

That said, I do worry a bit that with AI we’ll see less actual thinking and more defaulting to whatever is suggested. I’m a bit afraid we’ll end up with very “cemented” tech choices. I’ve actually been meaning to write a post about this for a month now… but haven’t found the time yet 😅
&nbsp; [[Image: leob profile image]](https://dev.to/leob) [leob](https://dev.to/leob) leob [leob](https://dev.to/leob) Follow
- Pronouns he/him
- Joined Aug 3, 2017
• [Apr 3 • Edited on Apr 4 • Edited](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fleob%2Fcomment%2F36cdo)

Thanks! Yes that's my fear as well, that everything will a bit more "canned" and run-of-the-mill!

I am seriously of the opinion, among all the AI "hype", that it wouldn't be a bad thing if some development is still done "the old-fashioned way", if only as an antidote, or no, let me make that more specific:

- some code is just fun to write, so why not write it your self?
- keep the old craft alive, for yourself and for mankind
- create code/patterns which can serve as a good (or preferred) "templates" for AI to copy - and 'original' material for AI models to be trained on!
- and to use or 'promote' different ways of doing something, including using other frameworks than the dominant ones
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 4](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36d73)

These are really great ideas 🙌 I especially like the angle of keeping the “craft” alive — not everything has to be automated, some things are just genuinely fun to build.

And your last point actually reminded me of something a friend of mine does. She noticed that AI can sometimes go in completely the wrong direction, so now she always asks it to provide not just one solution, but also 2–3 alternatives. It’s surprisingly effective.

I’ve had similar situations myself — the LLM would suggest adding 10 files and building some complex structure, and when I asked “can this be simpler?”, it suddenly turned out that… yes, it can, and it’s just one extra line 😄
&nbsp; [[Image: leob profile image]](https://dev.to/leob) [leob](https://dev.to/leob) leob [leob](https://dev.to/leob) Follow
- Pronouns he/him
- Joined Aug 3, 2017
• [Apr 4](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fleob%2Fcomment%2F36d7i)

*"now she always asks it to provide not just one solution, but also 2–3 alternatives"* - nice one, need to remember that! &nbsp; [[Image: apex_stack profile image]](https://dev.to/apex_stack) [Apex Stack](https://dev.to/apex_stack) Apex Stack [Apex Stack](https://dev.to/apex_stack) Follow Apex Stack
- Joined Mar 9, 2026
• [Apr 2](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fapex_stack%2Fcomment%2F36bfk)

The `&lt;dialog&gt;` one hits hard. I recently audited a site where the modal library alone added 14KB gzipped to the bundle — for something the browser does natively with better accessibility out of the box.

The `loading="lazy"` point is especially relevant for anyone running image-heavy pages at scale. I manage a site with thousands of pages across 12 languages and switching from a JS lazy-loading library to native `loading="lazy"` shaved ~200ms off LCP on mobile. That's the kind of win that directly impacts Core Web Vitals scores.

Would love to see a follow-up on native CSS features that replace JS too — `scroll-snap`, `container queries`, and `@layer` have quietly eliminated a lot of JS-heavy patterns.
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 3](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36bm0)

Thanks a lot! 🙌 And that CSS-focused follow-up is a great idea, there’s definitely a lot to explore there 😄
&nbsp; [[Image: apex_stack profile image]](https://dev.to/apex_stack) [Apex Stack](https://dev.to/apex_stack) Apex Stack [Apex Stack](https://dev.to/apex_stack) Follow Apex Stack
- Joined Mar 9, 2026
• [Apr 3](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fapex_stack%2Fcomment%2F36c0i)

Right? The CSS side is where most of the hidden bloat lives. I've seen projects pulling in entire utility frameworks just for a handful of layout patterns that `grid` and `has()` handle natively now. Would love to see someone benchmark the real-world performance delta between CSS-native approaches and popular UI kits.
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 4](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36d74)

Haha true, that would be super interesting to see 😄 I’d love to see real numbers comparing CSS-native approaches vs full UI kits in real-world apps.
&nbsp; [[Image: apex_stack profile image]](https://dev.to/apex_stack) [Apex Stack](https://dev.to/apex_stack) Apex Stack [Apex Stack](https://dev.to/apex_stack) Follow Apex Stack
- Joined Mar 9, 2026
• [Apr 4](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fapex_stack%2Fcomment%2F36d89)

Would be a fascinating benchmark! On one of my projects I went Tailwind-only (no component library) for a data-heavy site with 100K+ pages — the CSS output is surprisingly small because utility classes deduplicate naturally at scale. The entire site ships under 30KB of CSS gzipped.

The hidden cost with full UI kits isn't just bundle size though — it's the cascade of JavaScript that comes with interactive components you probably don't need. A datepicker here, a modal there, and suddenly you're shipping 200KB of JS for what's essentially a static content site.

Would love to see someone do a Lighthouse comparison: same layout built with Tailwind vs MUI vs Chakra. My bet is the Tailwind version wins on CLS and LCP by a noticeable margin on mobile. &nbsp; [[Image: crisiscoresystems profile image]](https://dev.to/crisiscoresystems) [CrisisCore-Systems](https://dev.to/crisiscoresystems) CrisisCore-Systems [CrisisCore-Systems](https://dev.to/crisiscoresystems) Follow Protective computing engineer at CrisisCore-Systems. I build privacy-first, offline-capable software for people under pain, stress, instability, and real-world pressure.
- Email [crisiscore.systems@proton.me](mailto:crisiscore.systems@proton.me)
- Location Kelowna, BC
- Education Self-taught engineer shaped by production failures, open source, and real-world instability.
- Pronouns he/him
- Work Founder &amp; protective computing engineer at CrisisCore-Systems
- Joined Nov 27, 2025
• [Apr 2](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fcrisiscoresystems%2Fcomment%2F36bdk)

This was a fun read because it hits a problem that quietly infects a lot of frontend work: people love building machinery for things the platform already handles perfectly well.

What I liked most is that this was not just another list of APIs for novelty points. The deeper point is discipline. A lot of overengineering starts with the assumption that custom automatically means better, when half the time it just means more code, more surface area, more bugs, and another dependency nobody needed in the first place.

That is why posts like this matter. They remind people that modern browsers are not some primitive canvas waiting to be rescued by JavaScript. They are already packed with capabilities that a lot of developers either forgot, never learned, or bypassed out of habit.

Also appreciated that this did not turn into anti library absolutism. That part matters. Libraries have their place, but too many teams reach for them before they have even asked the basic question: has the platform already solved this well enough?

Strong piece. The browser has grown up a lot, and a lot of developers are still coding like it has not.
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 3](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36bm5)

Exactly this. Custom doesn’t automatically mean better — and in many cases the browser has already handled edge cases we might not even think about in our own implementations. That’s often where custom solutions start to fall apart 😄
&nbsp; [[Image: crisiscoresystems profile image]](https://dev.to/crisiscoresystems) [CrisisCore-Systems](https://dev.to/crisiscoresystems) CrisisCore-Systems [CrisisCore-Systems](https://dev.to/crisiscoresystems) Follow Protective computing engineer at CrisisCore-Systems. I build privacy-first, offline-capable software for people under pain, stress, instability, and real-world pressure.
- Email [crisiscore.systems@proton.me](mailto:crisiscore.systems@proton.me)
- Location Kelowna, BC
- Education Self-taught engineer shaped by production failures, open source, and real-world instability.
- Pronouns he/him
- Work Founder &amp; protective computing engineer at CrisisCore-Systems
- Joined Nov 27, 2025
• [Apr 3](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fcrisiscoresystems%2Fcomment%2F36cdc)

Exactly. That is usually where the illusion breaks.

A custom solution often looks clean when it is first demoed because it only reflects the cases the developer remembered to design for. The browser implementation has usually already been dragged through the ugly reality of focus behavior, accessibility, input methods, device quirks, resizing, timing, and all the other little failure paths people do not think about until users hit them in production.

That is why platform discipline matters so much. Not because custom work is always wrong, but because a lot of teams are quietly volunteering themselves to re-solve problems the browser has already spent years hardening.

That is also what I liked about your post. It was really a case for restraint. Knowing when not to build is just as important as knowing how to build. &nbsp; [[Image: crevilla2050 profile image]](https://dev.to/crevilla2050) [crevilla2050](https://dev.to/crevilla2050) crevilla2050 [crevilla2050](https://dev.to/crevilla2050) Follow
- Joined Mar 28, 2024
• [Apr 2](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fcrevilla2050%2Fcomment%2F36bf8)

Wow, lots of useful info that I barely knew about. Time to start exploring more browser capabilities. And I am an avid reader of Sci-fi, I would love to read your story if you care to share. With the project I am working, Dennis the Forge, I am also writing a story parallel to the development, to vent my artistic side (I am both an artist and a programmer). Now I am feeling the itch to go write some code and test all these cool features you mentioned. Great article! Saludos.
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 3](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36blo)

Thank you so much! 😊 I actually have a feeling that writing sci-fi and programming aren’t that separate after all,especially for someone with an artistic side. Both are about imagining systems, worlds, and “what if” scenarios, just expressed in different ways. There’s just one tiny catch though: my story is in Polish 😄 When it comes to blog posts, my English + a bit of GPT polishing works just fine, but unfortunately it doesn’t quite translate the same way for prose 😅
&nbsp; [[Image: crevilla2050 profile image]](https://dev.to/crevilla2050) [crevilla2050](https://dev.to/crevilla2050) crevilla2050 [crevilla2050](https://dev.to/crevilla2050) Follow
- Joined Mar 28, 2024
• [Apr 3](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fcrevilla2050%2Fcomment%2F36ce2)

That is unfortunate, even though years ago I had a Polish girlfriend, only a few words still stay in my memory. And I know what you mean with translations: I prefer reading the story in the original language it was written, I really dislike translations.
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 4](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36d76)

Exactly! The original is always the best. And when it comes to really good translations, those feel like a work of art on their own, almost as if a co-author was involved, not just a translator. There are books like that, but I feel like they’re becoming rarer. And I’m not surprised you only picked up a few Polish words 😄 It’s a really difficult language. I honestly don’t know how people from abroad manage to learn it! &nbsp; [[Image: htho profile image]](https://dev.to/htho) [Hauke T.](https://dev.to/htho) Hauke T. [Hauke T.](https://dev.to/htho) Follow (Vanilla-) JavaScript/TypeScript Developer and Usabilty Engineer. Always on the path to become a Clean Code Developer.
- Location Würzburg, Germany
- Work Usability Engineer &amp; Frontend Developer
- Joined Dec 4, 2019
• [Apr 2](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fhtho%2Fcomment%2F36ah6)

`requestAnimationFrame` is great. You probably can skip the `Date.now()` part. The callback receives a timestamp as the first argument: [developer.mozilla.org/en-US/docs/W...](https://developer.mozilla.org/en-US/docs/Web/API/Window/requestAnimationFrame)
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 2](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36aib)

Good catch, you’re right! Thanks for the clarification 🙂 &nbsp; [[Image: trinhcuong-ast profile image]](https://dev.to/trinhcuong-ast) [Kai Alder](https://dev.to/trinhcuong-ast) Kai Alder [Kai Alder](https://dev.to/trinhcuong-ast) Follow Building dev tools. Shipping fast.
- Joined Jan 28, 2026
• [Apr 3](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Ftrinhcuong-ast%2Fcomment%2F36c17)

The `crypto.getRandomValues` one really resonates. I've seen so many projects use `Math.random()` for session tokens or even cart IDs - then wonder why they're getting collisions in production at scale. The browser's crypto API isn't just "better entropy" - it's the difference between "works in testing" and "works with real traffic."

Also worth mentioning `crypto.randomUUID()` if you just need a standard UUID v4 - one-liner and doesn't need the byte array dance. Been using it for a while now and it's surprisingly well-supported.

Curious what made you pick these 9 specifically? Did they all come from real audits or was it more "things that annoyed you enough to write about"? 😄
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 3](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36c28)

Thanks a lot for the crypto clarification — great addition! 🙌 And yes, crypto.randomUUID() is such a nice one-liner, definitely worth mentioning too.

As for the 9… probably the second option 😄 I basically started thinking about lesser-known features and just kept listing them. Honestly, I could keep going for quite a while, but figured I can always do a part two 😏 &nbsp; [[Image: htho profile image]](https://dev.to/htho) [Hauke T.](https://dev.to/htho) Hauke T. [Hauke T.](https://dev.to/htho) Follow (Vanilla-) JavaScript/TypeScript Developer and Usabilty Engineer. Always on the path to become a Clean Code Developer.
- Location Würzburg, Germany
- Work Usability Engineer &amp; Frontend Developer
- Joined Dec 4, 2019
• [Apr 2 • Edited on Apr 2 • Edited](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fhtho%2Fcomment%2F36ahf)

I like `requestIdleCallback`. I read about a pattern you can implement with it: idle-until-urgent: There are some expensive calculations required for some function the user is likely to invoke. Instead of doing it once it is required, you can do it once you have time for it. Basically pre-caching. But: when the result is required before the work was done, you cancel the callback and do it just-in-time.
&nbsp; [[Image: sylwia-lask profile image]](https://dev.to/sylwia-lask) [Sylwia Laskowska](https://dev.to/sylwia-lask) Sylwia Laskowska [Sylwia Laskowska](https://dev.to/sylwia-lask) Follow Software dev • 10+ yrs of code &amp; caffeine ☕ • Sci-fi fan • Bug whisperer 🐞
- Location Gdansk, Poland
- Joined Sep 28, 2025
• [Apr 2](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- [Copy link](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99)
- Hide
- [Report abuse](https://dev.to/report-abuse?url=https%3A%2F%2Fdev.to%2Fsylwia-lask%2Fcomment%2F36ai9)

Exactly this! It’s such a beautiful pattern. Idle-until-urgent really changes how you think about work scheduling in the browser. Instead of “do everything immediately”, it becomes “do it when it makes sense… unless the user needs it now”. Feels like a small thing, but it actually shifts your whole mental model of what the browser can handle for you today. [View full discussion (72 comments)](https://dev.to/sylwia-lask/9-things-youre-overengineering-the-browser-already-solved-them-o99/comments)
For further actions, you may consider blocking this person and/or [reporting abuse](https://dev.to/report-abuse)
[[Image: profile] Sonar](https://dev.to/sonar) Promoted
- [What's a billboard?](https://dev.to/billboards)
- [Manage preferences](https://dev.to/settings/customization)
- [Report billboard](https://dev.to/report-abuse?billboard=259978)

[[Image: State of Code Developer Survey report]](https://www.sonarsource.com/sem/the-state-of-code/developer-survey-report/?amp%3Butm_source=dev&amp%3Butm_campaign=ss-state-of-code-developer-survey26&amp%3Butm_content=report-devsurvey-banner-x-2&amp%3Butm_term=ww-all-x&amp%3Bs_category=Paid&amp%3Bs_source=Paid+Social&amp%3Bs_origin=dev&amp%3Bbb=259978)

## [#state-of-code-developer-survey-report](#state-of-code-developer-survey-report) [State of Code Developer Survey report](https://www.sonarsource.com/sem/the-state-of-code/developer-survey-report/?amp%3Butm_source=dev&amp%3Butm_campaign=ss-state-of-code-developer-survey26&amp%3Butm_content=report-devsurvey-banner-x-2&amp%3Butm_term=ww-all-x&amp%3Bs_category=Paid&amp%3Bs_source=Paid+Social&amp%3Bs_origin=dev&amp%3Bbb=259978)

Did you know 96% of developers don't fully trust that AI-generated code is functionally correct, yet only 48% always check it before committing? Check out Sonar's new report on the real-world impact of AI on development teams.

[Read the results](https://www.sonarsource.com/sem/the-state-of-code/developer-survey-report/?amp%3Butm_source=dev&amp%3Butm_campaign=ss-state-of-code-developer-survey26&amp%3Butm_content=report-devsurvey-banner-x-2&amp%3Butm_term=ww-all-x&amp%3Bs_category=Paid&amp%3Bs_source=Paid+Social&amp%3Bs_origin=dev&amp%3Bbb=259978)