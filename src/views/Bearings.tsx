/**
 * Roulements — la portée d'arbre.
 *
 * ## Ce que cet écran doit rendre visible
 *
 * Le résultat croise **deux natures de source**, et les confondre tromperait :
 *
 *  - la **classe de tolérance** (k5, m6, g6…) est une *recommandation de
 *    fabricant*. Aucune norme ne l'impose, et s'en écarter reste légitime si la
 *    raison en est connue ;
 *  - les **écarts** qui en découlent sont *normatifs* : ils viennent de
 *    l'ISO 286, confrontée à la norme elle-même.
 *
 * La frontière passe au milieu du résultat. L'écran la matérialise : deux
 * panneaux distincts, chacun avec son étiquette d'état de source, et le
 * raisonnement du moteur qui pose les deux étapes côte à côte. Présenter le tout
 * d'un bloc durcirait la recommandation en règle.
 *
 * ## Ce que cet écran ne fait pas
 *
 * Il ne devine pas une désignation. « 6203 » se lit série 62 + symbole
 * d'alésage 03, mais « 623 » se lit série 62 + symbole 3, et la source ne dit
 * pas comment trancher : le moteur rend **les lectures possibles**, et c'est
 * l'utilisateur qui reconnaît la sienne.
 */

import { useCallback, useEffect, useState } from "react";

import { bearingAdvise, bearingCatalogue, bearingOptions, bearingRead } from "../api";
import { Select } from "../components/Select";
import { FeatureBlock } from "../components/FeatureBlock";
import { SourceTag, Sources } from "../components/Sources";
import { StatusBox } from "../components/StatusBox";
import { Why } from "../components/Why";
import { deviation, nominal } from "../format";
import type {
  AppError,
  BearingCatalogue,
  DesignationReading,
  MountingAdvice,
  MountingOption,
} from "../types";

/**
 * Une majuscule en tête de phrase.
 *
 * Le moteur rend sa phrase en minuscule parce qu'elle s'insère au milieu de la
 * conclusion (« Sur l'arbre : serrage de… »). En titre de panneau, elle prend
 * la majuscule : c'est de la typographie, pas une règle.
 */
function capitalise(phrase: string): string {
  return phrase.charAt(0).toUpperCase() + phrase.slice(1);
}

export function Bearings() {
  const [catalogue, setCatalogue] = useState<BearingCatalogue | null>(null);
  const [designation, setDesignation] = useState("6210");
  const [readings, setReadings] = useState<DesignationReading[] | null>(null);
  const [bore, setBore] = useState("50");
  const [family, setFamily] = useState("");
  const [regime, setRegime] = useState("");
  const [options, setOptions] = useState<MountingOption[] | null>(null);
  const [advice, setAdvice] = useState<MountingAdvice | null>(null);
  const [error, setError] = useState<AppError | null>(null);

  useEffect(() => {
    bearingCatalogue()
      .then((loaded) => {
        setCatalogue(loaded);
        // Le premier de chaque liste, pour que l'écran soit utilisable sans
        // avoir à répondre à trois questions avant de voir quoi que ce soit.
        // Aucun choix par défaut n'est inventé : ce sont ceux du moteur.
        setFamily((current) => current || (loaded.families[0]?.id ?? ""));
        setRegime((current) => current || (loaded.regimes[0]?.id ?? ""));
      })
      .catch((cause) => setError(cause as AppError));
  }, []);

  // Les cas d'emploi dépendent des trois entrées : dès qu'elles tiennent, on
  // les demande. C'est une lecture de table, pas un calcul lourd.
  useEffect(() => {
    if (family === "" || regime === "" || bore.trim() === "") {
      setOptions(null);
      return;
    }
    let cancelled = false;
    bearingOptions(regime, family, bore)
      .then((loaded) => {
        if (!cancelled) {
          setOptions(loaded);
          setError(null);
        }
      })
      .catch((cause) => {
        if (!cancelled) {
          setOptions(null);
          setError(cause as AppError);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [family, regime, bore]);

  const read = useCallback(async () => {
    if (designation.trim() === "") return;
    try {
      setReadings(await bearingRead(designation));
      setError(null);
    } catch (cause) {
      setReadings(null);
      setError(cause as AppError);
    }
  }, [designation]);

  const advise = useCallback(
    async (condition: string) => {
      try {
        setAdvice(await bearingAdvise(regime, condition, family, bore));
        setError(null);
      } catch (cause) {
        setAdvice(null);
        setError(cause as AppError);
      }
    },
    [regime, family, bore],
  );

  const chosenRegime = catalogue?.regimes.find((option) => option.id === regime);
  const chosenFamily = catalogue?.families.find((option) => option.id === family);

  return (
    <div className="stack">
      <div className="page-header">
        <h1>Roulements</h1>
        <p className="lead">
          Quel alésage se cache derrière une désignation, et quelle tolérance porter sur la portée
          d'arbre.
        </p>
      </div>

      {/* La réserve avant la saisie, comme pour la géométrie : l'une de ces
          sources n'est pas normative du tout, et c'est à savoir avant de s'en
          servir, pas après. */}
      {catalogue?.warnings.map((warning) => (
        <div key={warning} className="status caution" role="note">
          <span className="status-badge" aria-hidden="true">
            🟠
          </span>
          <div>
            <div className="status-headline">Origine des données</div>
            <div className="status-detail">{warning}</div>
          </div>
        </div>
      ))}

      <form
        className="card"
        onSubmit={(event) => {
          event.preventDefault();
          void read();
        }}
      >
        <header>
          <span className="card-title">Le roulement</span>
        </header>

        <div className="row" style={{ alignItems: "flex-start" }}>
          <div className="grow">
            <label htmlFor="bearing-designation">Désignation</label>
            <input
              id="bearing-designation"
              type="text"
              value={designation}
              placeholder="6210"
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => setDesignation(event.target.value)}
            />
            <p className="hint">
              MecaTool ne tranche pas à votre place : il rend les découpages possibles de la
              désignation, et vous reconnaissez le vôtre.
            </p>
          </div>

          <button type="submit" className="btn">
            Lire la désignation
          </button>

          <div style={{ minWidth: "170px" }}>
            <label htmlFor="bearing-bore">Alésage (mm)</label>
            <input
              id="bearing-bore"
              type="text"
              value={bore}
              placeholder="50"
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => setBore(event.target.value)}
            />
            <p className="hint">Ou saisissez-le directement, sans désignation.</p>
          </div>
        </div>
      </form>

      {readings ? (
        <section className="card">
          <header>
            <span className="card-title">Lectures possibles</span>
            <span className="faint">
              {readings.length} découpage{readings.length > 1 ? "s" : ""} compatible
              {readings.length > 1 ? "s" : ""} de « {designation} »
            </span>
          </header>

          {readings.length === 0 ? (
            <p className="muted">
              Aucun découpage de cette désignation ne mène à un symbole d'alésage connu de la
              source.
            </p>
          ) : (
            <div className="table-scroll">
              <table>
                <caption className="visually-hidden">
                  Découpages possibles de la désignation saisie
                </caption>
                <thead>
                  <tr>
                    <th scope="col">Série</th>
                    <th scope="col">Symbole</th>
                    <th scope="col" className="num">
                      Alésage
                    </th>
                    <th scope="col">Lecture</th>
                    <th scope="col">
                      <span className="visually-hidden">Retenir</span>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {readings.map((reading) => (
                    <tr key={`${reading.series}-${reading.bore_code}`}>
                      <th scope="row" className="mono">
                        {reading.series}
                      </th>
                      <td className="mono">{reading.bore_code}</td>
                      <td className="num">{nominal(reading.bore)} mm</td>
                      <td>{reading.explanation}</td>
                      <td>
                        <button
                          type="button"
                          className="btn-quiet"
                          onClick={() => setBore(nominal(reading.bore))}
                        >
                          Retenir
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      ) : null}

      {catalogue ? (
        <section className="card">
          <header>
            <span className="card-title">Conditions de montage</span>
          </header>

          <div style={{ maxWidth: "420px", marginBottom: "var(--s-6)" }}>
            {/* La même liste que partout ailleurs. Quatre familles seulement, donc
                la recherche n'y sert guère — mais une liste qui se manie autrement
                que ses voisines coûte plus cher à l'usage que le champ de
                recherche ne coûte à l'écran. Les noms viennent du moteur. */}
            <Select
              id="bearing-family"
              label="Type de roulement"
              options={catalogue.families.map((option) => ({
                value: option.id,
                label: option.name,
              }))}
              value={family === "" ? null : family}
              onChange={(value) => {
                setFamily(value);
                setAdvice(null);
              }}
              placeholder="Choisir un type…"
            />
            {chosenFamily?.note ? <p className="hint">{chosenFamily.note}</p> : null}
          </div>

          <div className="field">
            <span className="field-label" id="regime-label">
              Régime de charge de la bague intérieure
            </span>
            {/* Le critère décisif du tableau. Chaque régime porte son
                explication : c'est elle qui permet de choisir, pas son nom. */}
            <div className="choices" role="group" aria-labelledby="regime-label">
              {catalogue.regimes.map((option) => (
                <button
                  key={option.id}
                  type="button"
                  className="choice"
                  aria-pressed={option.id === regime}
                  onClick={() => {
                    setRegime(option.id);
                    setAdvice(null);
                  }}
                >
                  <span className="choice-name">{option.name}</span>
                  <span className="choice-note">{option.explanation}</span>
                </button>
              ))}
            </div>
          </div>
        </section>
      ) : null}

      {error ? (
        <div className="status incompatible" role="alert">
          <span className="status-badge" aria-hidden="true">
            🔴
          </span>
          <div>
            <div className="status-headline">{error.message}</div>
            {error.hint ? <div className="status-detail">{error.hint}</div> : null}
          </div>
        </div>
      ) : null}

      {options ? (
        <section className="card">
          <header>
            <span className="card-title">Cas d'emploi</span>
            <span className="faint">
              {chosenFamily?.name} — alésage {bore} mm
            </span>
          </header>

          <div className="table-scroll">
            <table>
              <caption className="visually-hidden">
                Cas d'emploi du régime retenu, et classe de tolérance de chacun
              </caption>
              <thead>
                <tr>
                  <th scope="col">Condition</th>
                  <th scope="col">Exemples</th>
                  <th scope="col">Classe</th>
                  <th scope="col">
                    <span className="visually-hidden">Choisir</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {options.map((option) => (
                  <tr key={option.condition}>
                    <th scope="row">{option.condition}</th>
                    <td className="muted">{option.examples}</td>
                    <td>
                      {option.class ? (
                        <strong className="mono">{option.class}</strong>
                      ) : (
                        // La source ne conclut pas ici : le dire, plutôt qu'un
                        // tiret muet qu'on lirait comme « rien à faire ».
                        <span className="muted">Non définie — {option.unavailable}</span>
                      )}
                    </td>
                    <td>
                      {option.class ? (
                        <button
                          type="button"
                          className="btn-quiet"
                          aria-pressed={advice?.condition === option.condition}
                          onClick={() => void advise(option.condition)}
                        >
                          Voir les écarts
                        </button>
                      ) : null}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      ) : null}

      {advice ? (
        <>
          {/*
            Les deux étapes, séparées. À gauche ce qui est recommandé, à droite
            ce qui est normatif : la frontière traverse le résultat, elle ne
            doit pas disparaître dans un panneau unique.
          */}
          <section className="card">
            <header>
              <span className="card-title">1 — Classe recommandée</span>
              <SourceTag provenance={advice.provenance} />
            </header>

            <p className="headline">
              <strong className="num">{advice.designation}</strong>
            </p>

            <dl className="rows wrap">
              <dt>Type de roulement</dt>
              <dd>{advice.family.name}</dd>
              <dt>Régime de charge</dt>
              <dd>{advice.regime.name}</dd>
              <dt>Cas d'emploi</dt>
              <dd>{advice.condition}</dd>
              <dt>Classe</dt>
              <dd className="mono">{advice.class}</dd>
            </dl>

            <p className="hint">{advice.examples}</p>
          </section>

          <section className="card">
            <header>
              <span className="card-title">2 — Écarts de cette classe</span>
              <SourceTag provenance={advice.shaft.provenance} />
            </header>

            <p className="hint">
              À partir d'ici, plus rien n'est recommandé : la classe une fois choisie, ses écarts
              se calculent.
            </p>

            <div className="grid-2" style={{ marginTop: "var(--s-6)" }}>
              <FeatureBlock tolerance={advice.shaft.tolerance} />
            </div>

            <div style={{ marginTop: "var(--s-6)" }}>
              <Why steps={advice.shaft.steps} label="Comment ces écarts sont obtenus" />
            </div>
          </section>

          {/*
            Troisième panneau : ce que les deux premiers ne pouvaient pas dire.
            La classe seule ne renseigne pas sur le montage — il faut savoir ce
            que l'arbre rencontre. L'alésage d'un roulement n'est pas h0 : il
            porte son propre écart normalisé, toujours négatif, d'où le serrage.
          */}
          <section className="card">
            <header>
              <span className="card-title">3 — Ajustement obtenu</span>
            </header>

            <p className="headline">
              <strong className="num">{capitalise(advice.fit.summary)}</strong>
            </p>

            <dl className="rows wrap">
              <dt>Alésage du roulement</dt>
              <dd>
                <span className="mono">{advice.bearing_bore.characteristic}</span>{" "}
                {deviation(advice.bearing_bore.deviations.lower)} à{" "}
                {deviation(advice.bearing_bore.deviations.upper)}
              </dd>
              <dt>Classe du roulement</dt>
              <dd>{advice.bearing_bore.tolerance_class}</dd>
              <dt>Échelon</dt>
              <dd>{advice.bearing_bore.range_label}</dd>
              <dt>Arbre {advice.class}</dt>
              <dd>
                {deviation(advice.shaft.tolerance.deviations.lower)} à{" "}
                {deviation(advice.shaft.tolerance.deviations.upper)}
              </dd>
            </dl>

            <p className="hint">
              {advice.bearing_bore.meaning}. Les deux moitiés du calcul sont normatives —
              ISO 286 pour l'arbre, ISO 492 pour le roulement ; seul le choix de la classe
              reste une recommandation.
            </p>
          </section>

          <section className="card">
            <header>
              <span className="card-title">Conclusion</span>
            </header>
            <StatusBox conclusion={advice.conclusion} collapsedWhy={false} />
          </section>

          <Sources provenance={advice.provenance} label="D'où viennent ces chiffres" />
        </>
      ) : null}

      {chosenRegime && !advice ? (
        <p className="faint">
          Choisissez un cas d'emploi pour voir la classe et les écarts qu'elle donne.
        </p>
      ) : null}
    </div>
  );
}
