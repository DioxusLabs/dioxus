/** @type {import('tailwindcss').Config} */
module.exports = {
  mode: "all",
  content: ["./src/**/*.{rs,html,css}", "./docs/**/*.html"],
  theme: {
    extend: {
      colors: {
        dxorange: "#E96020",
        dxblue: "#00A8D6",
        ghmetal: "#24292f",
        ghdarkmetal: "#161b22",
        // ideblack: "#222529",
        ideblack: "#0e1116",
        // ideblack: "#0a0a0a",
        // ideblack: "#0E1116",
      },
      fontFamily: {
        // sans: [`"Poppins"`, "sans-serif"],
        // sans: ["Arimo", "sans-serif"],
        // sans: ["Lexend", "sans-serif"],
        sans: ["Inter var", "sans-serif"],
      },
      boxShadow: {
        "3xl": "0 35px 60px -1ww5px rgba(0, 0, 0, 0.5)",
        cutesy: "0px 0px 40px -5px rgba(255, 255, 255, 0.2)",
        // cutesy: "0px 0px 30px -10px white",
        // cutesy: "0px 0px 30px -10px red",
        pop: "0px 0px 30px -10px rgba(0, 0, 0, 0.5)",
      },
      keyframes: {
        fadein: {
          from: { opacity: "0" },
          to: { opacity: "1" },
        },
      },
      animation: {
        "fadein-medium": "fadein 500ms ease-in-out forwards",
      },
    },
  },
  plugins: [],
};
