# Vägkarta — basswerk

Status: **v1–v5 klara.** Det här är vad som är gjort och vad som är kvar.

## Klart

| | Steg | Innehåll |
| --- | --- | --- |
| v1 | Grund | Parser för låttexten, syntes (kick, bas, ackord, pad, hats, clap, riser, impact), WAV-export, CLI |
| v2 | Mixer & nivåer | Bussar, produktionsnivå 1–5, kryssrutor, sidechain-pump, sparning i `ljud\installning.txt` |
| v3 | Rum & nivå | Reverb och delay som send-bussar, loudness-normalisering, ratt för effektmängd |
| v4 | Liv | Fills före drops, lead-lager med variation mellan drops, ratt för lead-nivå |
| v5 | Ut & in | Stem-export (en WAV per buss + effektreturer), uppspelning i fönstret |

Fönstret (egui) finns och är standalone: `target\release\basswerk.exe`.

## Klart efter v5

- **Skapa ny låt i fönstret** — ämne + textnivå + genre → opencode skriver texten,
  basswerk gör musiken, mappen laddas in automatiskt.
- **Genrer** — Raggardunk, Dansband, Techno, Trap, Synthwave, Hårdrock. Skickas med
  i prompten **och styr ljudet** via presets som läses från `Genre:`-raden.
- **Inställningarna delade** — app-inställningar i `%APPDATA%\basswerk\`, mix per
  låt i `<låt>\ljud\`.
- **Buss-rattar** — åtta volymsliders.
- **Batch-läge** — kör om alla låtar i `låtar\` med var sin mix.
- **Ackordfärg** — Ren / 7:a / +9 / Sus.
- **Rullning och tydligare flöde** i fönstret.

## Två fel vi hittade på vägen

- **`exportera_stems` saknade genre-preseten.** Mastern och stem-filerna skulle
  blivit olika filer. Upptäcktes genom att mäta bas-stemmen före och efter
  genrebyte — den rörde sig inte alls.
- **Summan delades med antalet toner** i `stab` och `pad`. Fler toner gav alltså
  *lägre* volym — färgning blev tystare istället för rikare (0,0339 → 0,0262 från
  treklang till +9). Nu delas med `√n`, vilket håller energin konstant.

## Kvar

### 1. Knyt ihop med skillen

`/låt` skriver texterna. Lägg till att den sedan kör basswerk på mappen, så att
ett enda kommando ger **ämne in → färdig låt ut**. Hör i kommandofilen, inte i
skillen — sökvägen till `basswerk.exe` är maskinspecifik, och skillen ska gå att
publicera.

### 2. Buss-rattar i fönstret

**KLAR.** Åtta sliders (kick, bas, ackord, pad, hats, clap, fx, lead) sparas i
`<låt>\ljud\installning.txt` som `gain_<namn>`. Verifierat: bas-stemmen gick från
topp 0,464 till 0,928 vid 0,60 → 1,20.

OBS: stem-filerna är råa, så drar du en buss över ~1,0 kan den klippa i sin egen
WAV även om mastern är mjukbegränsad.

### 3. Batch-läge

**KLAR.** Knappen "Generera alla låtar" kör om varje låt under `låtar\` med sin
egen mix, och statusraden visar `3/8 NAMN` medan den jobbar. Krävde att kanalen
från bakgrundstråden byttes från ett enda resultat till ett `Jobb`-enum med
`Framsteg`/`Klar`/`Fel`.

### 4. Sång och röst

**DELVIS LÖST 2026-10-17.** Engelsk sång fungerar via `utau-lyrics-mcp` → `.ust`
→ PMoU med Tanya. Se "Engelsk sång — fungerande recept" nedan.

Svensk sång går fortfarande inte — det finns ingen svensk DiffSinger-röstbank.
Inbyggd modell för textskrivning är avförd (se nedan).

### 5. Fler ackordfärger

**KLAR.** Fyra lägen i fönstret: Ren (treklang), 7:a, +9 och Sus. Sparas per låt
som `farg=` i `ljud\installning.txt`.

De sätts på ackordstabbarna och paden — bastonen är opåverkad, så låtens
grundton ändras inte.

## Avfört: inbyggd AI (2026-10-17)

Frågan var om en liten modell (1–3 GB) kunde bakas in för att skriva texten, så
programmet blir helt fristående. **Testat, och svaret är nej.**

Fyra modeller kördes mot samma prompt (kort spec + ett exempel + ämne och nivå):

| Modell | Format | Skriver eget? | Nivå 4 |
| --- | --- | --- | --- |
| Qwen2.5 1.5B (0,9 GB) | 8/10 sektioner | nej — ordsallad | nej |
| SmolLM2 1.7B (1,0 GB) | 10/10 | nej — kopierar exemplet ordagrant | nej |
| Llama 3.2 3B (1,9 GB) | 10/10 | ja, men fel ton | avväpnar allt |
| Qwen3.5 9B (6,3 GB) | 10/10 | ja, temat rätt | ja, men bruten svenska |

**Tre fynd:**

1. **Formatet är lätt, språket är svårt.** Alla fyra klarade struktur, taktantal,
   tidskoder, `TEMPO:` och `KEY:`. Det som brast var skrivandet — tvärtom mot
   hypotesen.
2. **Små modeller kopierar exemplet** istället för att följa instruktionen.
3. **De avväpnar hellre än vägrar.** Ingen sa nej; Llama 3B bytte ut allt
   sexuellt mot vaga känslor.

Även 9B:an — som är dubbelt så stor som målintervallet — gav hooken
"HOMME! HEMME!" mot skillens "TUNGAN! MELLAN BENEN…". Det är en klasskillnad,
och 9B:an tar dessutom 108–155 s per låt eftersom den resonerar först.

**Beslut:** ingen inbakad modell. Texten skrivs fortsatt av skillen, musiken av
basswerk. Modellerna och testfilerna är raderade.

## Engelsk sång — fungerande recept (2026-10-17)

**Bevisat fungerande.** Tanya sjunger engelska ord, exporterat och klart.

```
1. svensk låttext
2. översätt till sjungbar engelska (räkna stavelser)
3. ackord läses ur låttexten, t.ex. "Ackord: Em - C - G - D"
4. utau-lyrics-mcp  ->  .ust   (en not per stavelse, toner ur ackorden)
5. PMoU: Import UST -> Tanya Magicbeat + fonemiserare "English"
        -> File -> Export Audio -> Mixdown To Wav File
6. sången mixas in i basswerk
```

| | |
| --- | --- |
| `utau-lyrics-mcp` | `D:\pengar genom AI\utau-lyrics-mcp` |
| Skapa `.ust` | `python -m utau_lyrics_mcp song text.txt --chords "Em \| C \| G \| D" --key Em --output-dir ut` |
| Röstbanker (PMoU) | `C:\Users\mange\Documents\OpenUtau\Singers` |
| Loggar | `C:\Users\mange\Documents\OpenUtau\Logs` |

### Vad som var nyckeln

- **`.ust` i stället för `.ustx`** — äldre format, stabilt. `utauMCP`:s
  `.ustx`-skrivare kraschar i OpenUtau 0.1.575 (`UNote.Validate`, index utanför
  listan); den är byggd mot ett äldre schema.
- **`+`-spridning** — `between` får en not, `+` nästa. Så sprider OpenUtau ett ord
  över flera noter. Utan det blir det ett ord per not och låter fel.
- **Tanya + fonemiseraren "English"** — PRINTmov-röst kräver PRINTmov-fonemiserare.
  Generiska `DiffSingerPhonemizer` gav `phonemizer error` och tystnad.
- **Melodin ur ackorden** — inga noter behövde hittas på för hand.
- **Rätt spår** — noter på ett spår utan singer = tystnad, oavsett allt annat.

### Fällor vi gick i

| Symptom | Orsak |
| --- | --- |
| Helt tyst, ingen felkod | `mute: true` på spåret |
| 46 byte WAV | exporten renderade 0 sekunder |
| `Phoneme "m eh l" isn't supported` | fonemiseraren delar inte på mellanslag |
| `phonemizer error <ord>` | fel fonemiserare för rösten |
| `UNote.Validate` index | `.ustx` från utauMCP, äldre schema |
| Play-knappen gör inget | `"Beta": false` i `prefs.json` |

## Principer vi lärt oss på vägen


- **Mät vad som händer, ratta vad som låter.** RMS kan bevisa att ett lager
  spelas, men inte om det låter bra. Reverb och lead fick var sin ratt istället
  för fler gissningar.
- **Bygg fundamentet en gång.** Bussarna i v2 gav sidechain, stems och rattar
  nästan gratis — fyra av fem senare steg stod på dem.
- **Läs API:et, gissa inte.** Både `eframe 0.37` och `rodio 0.23` hade bytt API
  mot vad man kan tro. Källkoden i crate-registret är facit.

