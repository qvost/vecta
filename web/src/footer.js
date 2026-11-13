import gsap from 'gsap';

export function footerMarkup() {
  return `
    <footer class="creator-footer" aria-label="About the creator">
      <div class="footer-details">
        <div class="footer-credit">
          <span>Vecta is built by</span>
          <a href="https://github.com/qvost">Siddid Pandey</a>
          <span>Kathmandu, Nepal</span>
        </div>
        <nav class="footer-links" aria-label="Project links">
          <a href="https://github.com/qvost/vecta">Source code</a>
          <a href="https://github.com/qvost/vecta/issues">Report an issue</a>
        </nav>
        <nav class="footer-links" aria-label="Qvost's profiles">
          <a href="https://github.com/qvost">GitHub</a>
          <a href="https://www.linkedin.com/in/qvost/">LinkedIn</a>
          <a href="https://x.com/qvostlost">X / Twitter</a>
        </nav>
        <a class="footer-top" href="#report">Back to top <span aria-hidden="true">↑</span></a>
      </div>
      <a class="footer-signature" href="https://github.com/qvost" aria-label="Qvost on GitHub">
        <span aria-hidden="true">${[...'Qvost'].map(letter => `<span class="footer-letter">${letter}</span>`).join('')}</span>
        <svg viewBox="0 0 64 64" aria-hidden="true"><path d="M10 54 54 10M10 10h44v44"/></svg>
      </a>
      <div class="footer-bottom">
        <span>© ${new Date().getFullYear()} Qvost</span>
        <span>Vecta · MIT / Apache-2.0</span>
      </div>
    </footer>`;
}

export function animateFooter() {
  const media = gsap.matchMedia();
  media.add('(prefers-reduced-motion: no-preference)', () => {
    gsap.from('.footer-letter', {
      yPercent: 110,
      rotation: 6,
      stagger: 0.045,
      ease: 'power2.out',
      scrollTrigger: {
        trigger: '.creator-footer',
        start: 'top 95%',
        end: 'bottom bottom',
        scrub: 0.7,
      },
    });
  });
  return () => media.revert();
}
