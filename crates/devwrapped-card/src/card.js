"use strict";

const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
const number = /^(\d[\d,]*)(?:\.(\d))?([KMB]?)$/;
const main = document.querySelector("main");
const slides = [...document.querySelectorAll(".slide")];
const finals = new Map();
let current = 0;
let frames = [];

for (const value of document.querySelectorAll(".value")) {
  finals.set(value, value.textContent);
}

function format(value, decimals) {
  if (decimals) {
    return (Math.floor(value * 10) / 10).toFixed(1);
  }
  return Math.floor(value).toLocaleString("en-US");
}

function countUp(slide) {
  for (const value of slide.querySelectorAll(".value")) {
    const final = finals.get(value);
    value.textContent = final;
    const match = number.exec(final);
    if (reduced || !match) {
      continue;
    }
    const [, whole, tenths, suffix] = match;
    const target = Number(whole.replaceAll(",", "") + (tenths ? "." + tenths : ""));
    const part = value.closest(".part");
    const delay = parseFloat(getComputedStyle(part).getPropertyValue("--delay")) || 0;
    const start = performance.now() + delay;
    value.textContent = format(0, tenths) + suffix;
    const step = (now) => {
      const progress = Math.min(Math.max((now - start) / 800, 0), 1);
      if (progress === 1) {
        value.textContent = final;
        return;
      }
      value.textContent = format(target * (1 - 2 ** (-10 * progress)), tenths) + suffix;
      frames.push(requestAnimationFrame(step));
    };
    frames.push(requestAnimationFrame(step));
  }
}

function place() {
  slides.forEach((slide, index) => {
    slide.classList.toggle("before", index < current);
    slide.classList.toggle("current", index === current);
    slide.classList.toggle("after", index > current);
  });
}

// at either end the slide gives a little and springs back, so the tap is not lost
function nudge(direction) {
  if (reduced) {
    return;
  }
  slides[current].animate(
    [
      { transform: "translateX(0)" },
      { transform: `translateX(${-8 * direction}px)` },
      { transform: "translateX(0)" },
    ],
    { duration: 320, easing: "cubic-bezier(0.23, 1, 0.32, 1)" },
  );
}

function show(index, direction) {
  if (index < 0 || index >= slides.length) {
    nudge(direction);
    return;
  }
  if (index === current) {
    return;
  }
  frames.forEach(cancelAnimationFrame);
  frames = [];
  for (const [value, final] of finals) {
    value.textContent = final;
  }
  current = index;
  place();
  countUp(slides[current]);
}

main.addEventListener("click", (event) => {
  const direction = event.clientX < window.innerWidth / 3 ? -1 : 1;
  show(current + direction, direction);
});

document.addEventListener("keydown", (event) => {
  const moves = { ArrowRight: 1, " ": 1, Enter: 1, ArrowLeft: -1 };
  if (event.key in moves) {
    event.preventDefault();
    main.classList.toggle("instant", event.repeat);
    show(current + moves[event.key], moves[event.key]);
  } else if (event.key === "Home") {
    show(0, -1);
  } else if (event.key === "End") {
    show(slides.length - 1, 1);
  }
});

document.addEventListener("keyup", () => main.classList.remove("instant"));

// the first slide enters too, once the page has painted it hidden
slides[0].classList.remove("current");
current = 0;
slides.forEach((slide, index) => slide.classList.toggle("after", index > 0));
requestAnimationFrame(() =>
  requestAnimationFrame(() => {
    place();
    countUp(slides[current]);
  }),
);
