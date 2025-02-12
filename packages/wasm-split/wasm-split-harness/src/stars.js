// Handle caching of github stars
// This file is part of the fuzz test to prove we can handle linked js files

// Two days
const STAR_EXPIRE_TIME = 172800000;

export function get_stars(name) {
  let item = localStorage.getItem(name);
  let data = JSON.parse(item);

  if (!data) {
    return null;
  }

  if (data.expires <= Date.now()) {
    localStorage.removeItem(name);
    return null;
  }

  return data.stars;
}

export function set_stars(name, value) {
  let expires = Date.now() + STAR_EXPIRE_TIME;
  let data = { stars: value, expires };

  let converted = JSON.stringify(data);
  localStorage.setItem(name, converted);
}
