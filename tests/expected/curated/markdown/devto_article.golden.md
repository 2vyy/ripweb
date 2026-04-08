# 9 Things You’re Overengineering (The Browser Already Solved Them)
Sylwia Laskowska Posted on Apr 2

I love writing philosophical essays — thoughts about code, work, all that stuff. I also love deep technical dives. But I know you love my lists of cool features that not everyone has heard about yet 😄

What’s up with me? This week I’m preparing for a conference, fighting performance issues, and trying to get at least somewhat ready for the upcoming holidays 😉 

Something nice happened too. I enjoy writing — not just technical articles, but in general. Last summer my life changed quite a bit, and to keep my sanity I started writing a sci-fi story, which I submitted to a Polish science fiction foundation competition. I didn’t win, but my story made it pretty far — around 13th place out of 179 submissions. Considering it was my first attempt at this kind of writing… it could have gone worse 😄

And speaking of sci-fi — the kind happening right in front of us 😉 Today I’ve prepared a batch of things the browser can already do, which honestly didn’t fit in my head not that long ago. A lot of these are still not that widely known, and yet many of them are already supported across modern browsers. Have fun!

##  1. “Let me just run this later” → `requestIdleCallback`

At first I thought this API was pointless. It basically lets you run some code when nothing interesting is happening. Ok… cool… but why would I care?

Turns out — there are tons of use cases. For example, collecting data about how the user behaves on your page — definitely not something you want to do while your 200 components are rendering 😅 Or loading less important data, preprocessing something, generating images in the background.

Honestly, there are probably as many use cases as there are developers.

```js
function trackUserScrolling() {
  console.log("User scrolled. This changes everything.");
}

if ("requestIdleCallback" in window) {
  requestIdleCallback(trackUserScrolling);
} else {
  setTimeout(trackUserScrolling, 0);
}
```

Support: modern browsers (historically missing in Safari, so fallback is still a good idea) 

## 2. “Why is my input not highlighting???” → `:focus-within`

It’s easy to style an element that has focus. But what if you want to style the parent div? For example, make it pink, add some flowers 😉 You can write 40 lines of JavaScript… or just use `:focus-within`.

Works. No listeners. No bugs. No suffering.

```css
.form-field {
  border: 1px solid #ccc;
  padding: 12px;
}

.form-field:focus-within {
  border-color: hotpink;
}
```

```html
<div class="form-field">
  <input placeholder="Type something meaningful..." />
</div>
```

Support: basically everywhere that matters 

## 3. “Let’s show offline mode” → `navigator.onLine`

Have you ever built a PWA? Because I have, and the eternal problem is what to do when the user loses connection (e.g. they’re in the wilderness or just walked into an elevator 😄). You can write a bunch of complicated ifs, or just listen to `offline` and `online`. On `offline` you can store data in IndexedDB, and when the user is back online, send it to the server.

```js
window.addEventListener("offline", () => {
  alert("You are offline. Time to panic.");
});

window.addEventListener("online", () => {
  alert("You're back. Panic cancelled.");
});
```

Support: widely supported (but “online” ≠ “your backend works” 😅)

4. “Smooth animation, but make it cursed” → `requestAnimationFrame`

We’ve all seen this:

```js
setInterval(() => {
  element.style.left = Math.random() * 100 + "px";
}, 16);
```

You can feel this is not the best idea 😉 It just lags. Luckily we have `requestAnimationFrame`, which is synced with the browser repaint cycle, so things are actually smooth.

```js
function animate() {
  element.style.transform = `translateX(${Date.now() % 300}px)`;
  requestAnimationFrame(animate);
}

requestAnimationFrame(animate);
```

Support: everywhere

5. “This card should adapt… but only here” → container queries

This feature feels almost unfair. I’m at a point in my career where I barely write CSS anymore (well, except for occasional moments like the one I described here: Is learning CSS a waste of time in 2026?[1]).

But there was a time when I wrote a lot of it. And wow — how much I would have given to apply media queries to a specific element instead of the whole viewport. Now we finally can. The component becomes self-aware, and we can go grab a coffee.

```css
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

Support: modern browsers (add fallback if needed)

## 6. “Random ID, what could go wrong?” → `crypto.getRandomValues`

```js
const id = Math.random().toString(36).slice(2);
```

This is how bugs are born. It looks like “good enough” crypto from AliExpress and works… until it doesn’t. First of all, it depends on the engine implementation — we don’t really know what’s happening under the hood. Some patterns are absolutely possible, and with enough IDs you’re basically asking for duplicates.

Luckily, we now have a simple native solution. It’s not a silver bullet, but `crypto.getRandomValues` is pretty solid — much better entropy, no weird patterns, dramatically reduces the chance of collisions. The browser just does it properly.

```js
const bytes = new Uint8Array(8);
crypto.getRandomValues(bytes);

const id = Array.from(bytes)
  .map(b => b.toString(16).padStart(2, "0"))
  .join("");

console.log("Secure-ish ID:", id);
```

Support: widely supported

7. “We need a modal” → `<dialog>`

It’s honestly nice that browsers finally stepped up and said: fine, here’s your modal 😄 No more installing 12KB libraries just to open a dialog that users love so much. This one is also accessible by default, so win-win.

```html
<dialog id="modal">
  <p>Are you sure you want to deploy on Friday?</p>
  <button onclick="modal.close()">Cancel</button>
  <button onclick="alert('Good luck 😬')">Deploy</button>
</dialog>

<button onclick="modal.showModal()">Open modal</button>
```

Support: modern browsers 

8. “Voice input would be cool…” → Speech API

Are you already installing transformers.js because you need speech recognition? Relax — turns out the browser has something for that too. Well… at least Chromium does 😄 So if you can “encourage” users to use Chrome, Edge, or something similar, you’re good. Personally, I’d still be careful with production use, but for demos? Why not.

```js
const SpeechRecognition =
  window.SpeechRecognition || window.webkitSpeechRecognition;

if (SpeechRecognition) {
  const recognition = new SpeechRecognition();

  recognition.onresult = e => {
    console.log("You said:", e.results[0][0].transcript);
  };

  recognition.start();
}
```

Support: mostly Chromium 

## 9. “Will this CSS explode?” → `@supports`

Here’s a modern solution to the classic “it works on my machine” — at least in CSS 😉 You don’t have to guess whether something will break your layout. Just wrap it in `@supports`. There is a small catch — while support is very good, it’s not literally everywhere, so ironically… we could use `@supports` for `@supports`.

```css
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

Support: very good 

## ⚠️ But don’t get me wrong

Libraries are great. Sometimes you absolutely need them. But sometimes… you’re installing a dependency for something the browser solved years ago. Before installing anything, just ask yourself (or Google): “Is the browser already smarter than me here?” Sometimes the answer is yes. And that’s… perfectly fine 😄

[1]: https://dev.to/sylwia-lask/is-learning-css-a-waste-of-time-in-2026-nj3