+++
title = "Preguntas puente"
+++

Estas preguntas cierran la Semana 1 y tienden el puente hacia la Semana 2.
Conviene pensarlas al terminar esta sesión, cuando el ambiente todavía está
fresco. No se buscan las respuestas de inmediato: se razonan desde lo que se
construyó, y anticipan lo que la próxima semana abre en detalle. Al comenzar la
primera sesión de la Semana 2, cada participante comparte su respuesta y se
discute en conjunto antes de continuar.

:::slide
## Preguntas puente

1. CloudFormation creó los recursos en un orden, y los borró en el orden
   inverso: ¿cómo sabe ese orden, si el template no lo indica?
2. Para cambiar un detalle del ambiente (una variable, un puerto), ¿borrar y
   recrear el stack es la única opción?
3. Las dos versiones del template difieren solo en la red: ¿qué sugiere eso
   sobre cómo partir el ambiente en piezas?
:::

---

## Pregunta 1

En la pestaña **Events** se vio a CloudFormation crear los recursos en un orden
preciso: la VPC antes que las subredes, el listener antes que el servicio. Y
borrarlos en el orden inverso. El template no tiene ninguna lista de pasos:
¿cómo sabe CloudFormation ese orden?

::: solucion
El orden no está escrito: se **deduce de las referencias entre recursos**. Cuando
un recurso menciona a otro (el servicio usa el grupo de destino, el grupo de
destino vive en la VPC), CloudFormation registra esa dependencia y construye un
grafo completo: lo referenciado se crea primero, y en el borrado el orden se
invierte. Para los pocos casos donde la dependencia existe pero no hay
referencia, el template la declara de forma explícita.

Ese es exactamente el contenido que se abre la próxima semana: al leer el
template recurso por recurso aparecen esas referencias (`!Ref`, `!GetAtt`) y la
dependencia explícita (`DependsOn`), y el orden de la pestaña **Events** pasa
a tener sentido.
:::

---

## Pregunta 2

El ciclo que se practicó fue borrar y recrear el stack completo. Pero si lo que
se necesita es cambiar un solo detalle del ambiente (una variable de entorno,
el comportamiento de un puerto), ¿destruir todo es la única opción? ¿Qué se
esperaría poder hacer?

::: solucion
No. CloudFormation permite **actualizar** un stack existente: se le entrega el
template (igual o modificado) con nuevos valores de parámetros, compara el
estado deseado con el actual, y aplica **solo la diferencia**. La mayoría de los
cambios se hacen en el lugar; algunos exigen reemplazar un recurso, y
CloudFormation lo indica antes de tocar nada mediante un *change set*: una
vista previa del cambio.

Quien hizo la sección opcional de HTTPS ya ejecutó una actualización así: el
parámetro `RedirigirAHttps` cambió a `si` y CloudFormation solo modificó la
acción del listener HTTP; el resto del ambiente ni se enteró.

La próxima semana se practica ese flujo con calma: change sets, qué pasa cuando
un cambio falla (*rollback*), y qué pasa cuando alguien toca los recursos por
fuera del template (*drift*).
:::

---

## Pregunta 3

Las dos versiones del template son idénticas salvo en una cosa: una crea la red
y la otra la recibe como parámetros. Y con la variante, al borrar el stack la
red sobrevive porque no le pertenece. ¿Qué sugiere eso sobre cómo organizar un
ambiente más grande: todo en un stack, o partido en piezas? ¿Con qué criterio se
partiría?

::: solucion
Sugiere que los recursos con **ciclos de vida distintos** conviene gestionarlos
en stacks distintos. La red casi nunca cambia y puede ser compartida; los datos
deben sobrevivir a los despliegues; la aplicación cambia todo el tiempo y es
descartable. Meter todo en un stack ata esas tres velocidades: borrar la
aplicación arrastra la red y los datos.

La variante de VPC existente ya insinuó la solución: la red vive fuera y el
stack la consume. Y quien hizo la sección opcional de HTTPS la vio completa:
ese stack agrega un listener al ALB de otro stack sin modificarlo, leyendo los
valores que el stack base **exporta**.

La próxima semana ese criterio se vuelve práctica: el ambiente se separa en
stacks de **red**, **datos**, y **aplicación**, conectados por ese mismo
mecanismo de exports e imports, y se comprueba que la aplicación se puede borrar
y recrear sin tocar los otros dos.
:::

---

## Dónde estamos

Al cerrar la Semana 1, cada participante tiene el flujo completo de la primera parte
del taller funcionando de punta a punta:

- Un **repositorio en CodeCommit** con el código de la aplicación, versionado con git.
- Un **pipeline de build en CodeBuild** que construye la imagen Docker y la publica
  en **ECR** a partir del `buildspec.yml`.
- La **aplicación en línea** sobre ECS/Fargate detrás de un ALB, desplegada con un
  template de CloudFormation, en su versión estándar o en la variante de VPC
  existente, según la cuenta.
- El **ciclo de recuperación** practicado: destruir y recrear el ambiente en minutos.
- Opcionalmente, la aplicación bajo un **dominio propio con HTTPS**, con un stack
  adicional que se conecta al stack base.

Se construyó, desplegó, y operó el sistema. Aún sin entender cómo se creó el template.

## Qué sigue en la Semana 2

La próxima semana se estudia CloudFormation más a fondo. Se va a:

- Leer el template de la Semana 1 recurso por recurso, y entender la
  **infraestructura como código**: parámetros, recursos, salidas, y funciones
  intrínsecas.
- **Actualizar** el stack de forma segura con *change sets*, y ver cómo CloudFormation
  maneja cambios, *drift*, y *rollback*.
- **Separar** el ambiente en stacks de red, datos, y aplicación, conectados por
  exports e imports.
- Conocer los **primeros contenedores** por dentro: las *task definitions* y los
  *services* de ECS/Fargate que el template creó.

Al final de la Semana 2 se comprenderá, y se podrá modificar, el ambiente que
esta semana solo se lanzó.
