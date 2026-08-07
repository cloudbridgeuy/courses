+++
title = "Introducción"
+++

:::inline-slide
## Bienvenida

El objetivo de este curso es presentar las herramientas que AWS ofrece para desplegar aplicaciones con buenas prácticas
de integración y despliegue continuos (`CI/CD`).

Primero definiremos los conceptos que usaremos durante el curso. Luego explicaremos la mecánica del taller y la forma
de trabajo para las próximas semanas.
:::

:::inline-slide light
## La narrativa del taller

:::skip
Durante cuatro semanas se desplegará y operará una aplicación web real de principio a fin,
sobre infraestructura de AWS. No se trata de ejercicios aislados: cada sesión avanza un paso
concreto de la misma historia. Al terminar la Semana 4, se contará con un flujo completo
para el despliegue de una aplicación, desde su desarrollo hasta su operación:
:::

:::add visibility=slide
Durante cuatro semanas se desplegará y operará una aplicación web real de principio a fin,
sobre infraestructura de AWS.
::: #add


| Servicio | Rol en el pipeline |
| --- | --- |
| CodeCommit | Repositorio de código fuente |
| CodeBuild | Construcción de la imagen |
| ECR | Registro de imágenes Docker |
| CloudFormation | Infraestructura como código |
| CodePipeline | Tareas de CI/CD |
| ECS + Fargate | Ejecución de contenedores |
| CloudWatch | Monitoreo y observabilidad |

:::

:::inline-slide light
## Objetivos del curso

1. Conversar sobre algunas de las responsabilidades y el lugar de DevOps en la organización.
2. Analizar cómo descomponer la Arquitectura de la empresa para su gestión.
3. Comprender cómo utilizar `Pipelines` para automatizar despliegues.
4. Comprender el espacio del Monitoreo y la Observabilidad.
:::

:::inline-slide
## Semana 1

La primera semana establece los cimientos que el resto del taller supone conocidos. Al
terminar la sesión del viernes se contará con:

:::skip
- Un **repositorio de código** en CodeCommit, con el código de la aplicación ya cargado.
- Un **pipeline de build** en CodeBuild que, cada vez que se ejecuta,
  compila la imagen y la publica en Amazon ECR.
- La **aplicación en línea**: accesible desde el navegador a través de un Application
  Load Balancer, desplegada con un template de CloudFormation provisto por el instructor,
  sobre ECS/Fargate, conectada a una tabla de DynamoDB.
:::

:::add visibility=slide
- Repositorio de código en `CodeCommit`.
- Pipeline de build en `CodeBuild`.
- Template de `CloudFormation` para el despliegue de la aplicación.
:::

::: info
El template de CloudFormation se usa esta semana como una caja negra: se lanza y se
obtiene un ambiente funcional en minutos. Cómo está construida por dentro es el tema de
la Semana 2.
::: #info
::: #inline-slide

:::inline-slide
## Semana 2

La segunda semana comienza interpretando el template de CloudFormation que en
la Semana 1 se lanzó sin mirar. Al terminar la semana:

:::skip
- Se podrá **leer y actualizar un template** de CloudFormation: parámetros, recursos,
  funciones intrínsecas y outputs, junto con las buenas prácticas y el troubleshooting
  de stacks.
- El monolito de la Semana 1 estará **separado en stacks por ciclo de vida** (red,
  datos, plataforma, aplicación), con la tabla de DynamoDB migrada mediante resource
  import, sin perder datos.
- Se habrán visto los **primeros contenedores**: las task definitions y los services
  de ECS/Fargate que el template definía, ya no como caja negra sino como recursos
  propios.
:::

:::add visibility=slide
- Anatomía de un template de `CloudFormation`.
- Separación de stacks por ciclo de vida + resource import.
- Primeros contenedores: task definitions y services de `ECS`/`Fargate`.
:::
::: #inline-slide

:::inline-slide
## Semana 3

Con la infraestructura descrita como código, la tercera semana pasa a operarla,
automatizarla y observarla. Al terminar la semana:

:::skip
- Se sabrá **operar los contenedores**: networking, escalabilidad y troubleshooting
  de ECS/Fargate.
- Un **pipeline de CI/CD** en CodePipeline conectará el push al despliegue, con una
  aprobación manual antes de producción y notificaciones de cada ejecución.
- Comenzará la **observabilidad**: métricas y logs en CloudWatch, con Container
  Insights activado sobre el clúster.
:::

:::add visibility=slide
- Operar `ECS`/`Fargate`: networking, escalabilidad, troubleshooting.
- Pipeline de CI/CD en `CodePipeline`, con aprobación manual.
- Observabilidad: métricas y logs en `CloudWatch`.
:::
::: #inline-slide

:::inline-slide
## Semana 4

La última semana, de una única sesión de tres horas, completa la observabilidad y
cierra el curso:

:::skip
- **Dashboards y alarmas** de CloudWatch sobre la aplicación desplegada.
- El **cierre del curso**: los mandamientos DevOps, los principios personales del
  instructor, los doce factores y los próximos pasos.
- El **teardown** final: desarmar el ambiente completo, en orden.
:::

:::add visibility=slide
- Dashboards y alarmas de `CloudWatch`.
- Cierre: mandamientos DevOps, doce factores, próximos pasos.
- Teardown del ambiente completo.
:::
::: #inline-slide

:::inline-slide
## El mecanismo de recuperación

:::skip
Desde el primer día se practica destruir el ambiente completo y recrearlo desde cero.
Esto no es un ejercicio de destrucción: es el seguro del taller. Si algo sale mal en
cualquier sesión posterior (una configuración equivocada, un recurso corrompido) se
borra el stack de CloudFormation, se espera a que termine, y se lo vuelve a lanzar con los mismos
parámetros, dejándolo en el punto de partida anterior. El costo de un error se reduce a
minutos de espera.
:::

:::add visibility=slide
Utilizamos `CloudFormation` desde el principio para asegurarnos de que podemos
reiniciar el pod en cualquier momento.
:::

::: warning
Si algo sale mal, borramos el `stack` de CloudFormation, esperamos a que termine, y lo volvemos a
lanzar con los mismos parámetros. El costo de un error se reduce a minutos de espera.
::: #warning
::: #inline-slide

:::inline-slide
## Cómo usar esta guía

Cada sección combina teoría breve con práctica guiada paso a paso. El esquema es
siempre el mismo:

:::skip
1. **Teoría**: el instructor presenta el problema del día y el servicio de
   AWS que lo resuelve. El instructor utilizará diapositivas directamente conectadas a
   esta guía, por lo que se puede seguir al instructor en cualquiera de las dos vistas.
2. **Práctica guiada**: seguir los pasos numerados en la guía. Intentar cada ejercicio
   antes de revelar la solución.
3. **Solución oculta**: si se producen bloqueos, pulsar el botón **Ver solución** bajo
   cada ejercicio. Aparecerán los clics exactos. Al final de cada sesión todos los
   participantes quedan en el mismo punto.
4. **Preguntas puente**: al terminar la sesión del miércoles (presencial), aparecen dos
   o tres preguntas que conectan lo construido con lo que viene el viernes (remota).
   Pensarlas antes de la siguiente sesión.
:::

:::add
1. **Teoría**
2. **Práctica Guiada**
3. **Ejercicios**
4. **Preguntas Puente**
::: #add
::: #inline-slide

:::inline-slide
## Requisitos previos

Antes de comenzar, confirmar que se cuenta con lo siguiente:

- **Cuenta AWS** con permisos para CodeCommit, CodeBuild, ECR, CloudFormation, ECS y
  Fargate, y DynamoDB. En caso de duda, consultar con el instructor.
- **Navegador web** actualizado (Chrome o Firefox recomendados).
- **Template de CloudFormation** `taller-aws-devops-semana1.yaml`, también provisto por el
  instructor. Se utiliza en la sección 4.
:::

---

:::inline-slide light
## ¿Qué es DevOps?

Una combinación de prácticas culturales, servicios y herramientas que aumentan la
capacidad de entregar software a alta velocidad.

El objetivo es unificar las áreas de desarrollo y operaciones, las cuales comúnmente
suelen estar completamente separadas, para conseguir un mejor flujo desde el código
a la aplicación desplegada.
:::

## La interfaz entre Desarrollo y Operaciones
:::inline-slide with-title

:::skip
Podemos representar la relación entre Desarrollo y Operaciones mediante una analogía con modelos por capas, como OSI
o núcleo/capa exterior (`core/shell`). Cada equipo concentra ciertas responsabilidades en su núcleo (`core`), pero
necesita una interfaz exterior (`shell`) para comunicarse con el otro.
:::

:::add visibility=slide
Podemos representar la relación entre Desarrollo y Operaciones mediante una analogía con modelos por capas, como OSI
o núcleo/capa exterior (`core/shell`).
:::

:::skip
Usaremos esta analogía para identificar los puntos de contacto y analizar las conversaciones que ambos equipos
necesitan para mantener un flujo continuo de información y evitar bloqueos.

Operaciones trabaja en la frontera entre la aplicación y su entorno de ejecución; no se limita a ejecutarla. Desarrollo
y Operaciones deben definir en conjunto cómo se expone la aplicación a los clientes. Estas decisiones incluyen los
protocolos de comunicación, la arquitectura, los chequeos de salud, el cifrado, los controles de seguridad y la
conectividad.
:::

:::add visibility=slide
Operaciones trabaja en la frontera entre la aplicación y su entorno de ejecución; no se limita a ejecutarla. Desarrollo
y Operaciones deben definir en conjunto cómo se expone la aplicación a los clientes.
:::

Ambos equipos deben participar en estas decisiones desde la definición del **artefacto desplegable**.
:::

:::inline-slide with-title light
### El artefacto desplegable

:::skip
El artefacto desplegable es la unidad que se entrega al entorno de ejecución. En el caso más simple, puede ser
el propio código fuente, entregado a Operaciones para su despliegue. Este modelo aparece con lenguajes interpretados o
de scripting, como Python, Ruby, PHP o JavaScript.
:::

:::add visibility=slide
El artefacto desplegable es la unidad que se entrega al entorno de ejecución.
:::

:::skip
Sin embargo, entregar solo el código fuente deja una pregunta abierta: ¿requiere la aplicación algún proceso de
construcción antes del despliegue? El código fuente resulta suficiente cuando el despliegue consiste en copiarlo y
ejecutarlo; en los demás casos, el proceso necesita una definición explícita.

Una opción más sólida incorpora un proceso de construcción descrito mediante un script o una tarea, como un
`Makefile`. Este proceso puede producir un directorio, un archivo `tar`, un paquete o un binario. El script no es el
artefacto desplegable: define una forma reproducible de generarlo.
:::

:::add visibility=slide
En el caso más simple, puede ser el propio código fuente, un script que incluya el proceso de construcción del
mismo (por ejemplo, un `Makefile`).
:::

Aun así, el resultado de la construcción no siempre describe el despliegue completo. El contrato de despliegue también
debe especificar:

1. **Configuración:** argumentos, variables de entorno y secretos.
2. **Dependencias:** runtimes, bibliotecas, paquetes del sistema y servicios externos de red.

:::skip
Desarrollo y Operaciones deben definir estos requisitos y acordar cómo los consume la aplicación. Muchas decisiones
dependen del entorno corporativo, su historia y las aplicaciones que ya están en producción. La solución correcta es
la que se ajusta a ese contexto, no necesariamente la más nueva, la más popular o la mejor en términos aislados.
:::

:::add visibility=slide
Desarrollo y Operaciones deben definir estos requisitos y acordar cómo los consume la aplicación.
:::

::: warning
Ambas decisiones generan fricción cuando Desarrollo queda fuera de las tareas del día 2 (no participa en la operación de los
sistemas en producción), y cuando Operaciones no tiene entrada en el día 1, durante el desarrollo del producto.
:::


::: #inline-slide

:::inline-slide light with-title
#### Diagrama de Operación y Desarrollo
```mermaid
%%{init: {"flowchart": {"defaultRenderer": "elk", "nodeSpacing": 30, "rankSpacing": 40, "padding": 8}, "themeVariables": {"clusterBkg": "#fffbeb", "clusterBorder": "#d97706"}}}%%
flowchart TD
    DCORE["Desarrollo · Núcleo<br/>Código · lógica de negocio · pruebas"]
    DSHELL["Desarrollo · Interfaz exterior<br/>Empaquetado · requisitos · instrumentación"]
    DCORE <--> DSHELL

    OCORE["Operaciones · Núcleo<br/>Infraestructura · seguridad · observabilidad · día 2"]
    OSHELL["Operaciones · Interfaz exterior<br/>Plataforma · políticas · capacidad · retroalimentación"]
    OCORE <--> OSHELL

    subgraph CONTRACT["Contrato de despliegue · interfaz compartida"]
        direction TB
        ART["Artefacto desplegable"]
        CFG["Configuración<br/>Argumentos · variables · secretos"]
        DEP["Dependencias<br/>Entorno de ejecución · bibliotecas · servicios"]
        EXP["Exposición<br/>Protocolos · puertos · cifrado · chequeos de salud"]

        ART ~~~ DEP
        CFG ~~~ EXP
    end

    DSHELL <-->|"Definición conjunta"| CONTRACT
    OSHELL <-->|"Validación operativa"| CONTRACT

    CONTRACT --> PIPELINE["Pipeline CI/CD"]
    PIPELINE --> PROD["Aplicación en producción"]

    DSHELL <-.-|"Retroalimentación de producción"| PROD
    OSHELL <-.-|"Métricas · logs · incidentes"| PROD

    classDef core fill:#dbeafe,stroke:#2563eb,color:#172554;
    classDef shell fill:#e0e7ff,stroke:#4f46e5,color:#1e1b4b;
    classDef contract fill:#fef3c7,stroke:#d97706,color:#451a03;
    classDef runtime fill:#dcfce7,stroke:#16a34a,color:#052e16;

    class DCORE,OCORE core;
    class DSHELL,OSHELL shell;
    class ART,CFG,DEP,EXP contract;
    class PIPELINE,PROD runtime;

    linkStyle 2,3 stroke:none;
    linkStyle 8,9 stroke-dasharray:5 4;
```
:::

:::inline-slide
## Imágenes como artefactos desplegables

Las imágenes de contenedor son artefactos de despliegue muy poderosos, porque
facilitan la gestión de las dependencias internas de la aplicación. El equipo
de Desarrollo puede definirlas y probarlas durante su propio flujo de trabajo
en un entorno representativo de producción.

::: warning
Sin embargo, adoptar imágenes como artefactos desplegables exige la
participación de Operaciones en su diseño. Ambos equipos deben acordar las
imágenes base, el tamaño, las actualizaciones, las prácticas de seguridad, los
puertos expuestos, el tiempo de construcción, el tiempo de arranque y el
rendimiento durante la ejecución.
:::

:::

:::inline-slide light
## Pipeline de entrega continua (CI/CD)

Una vez que tenemos definido el artefacto, tenemos que decidir cómo lo vamos a
poner en producción. Una forma sería ejecutar un `runbook` cada vez que se produce
una nueva versión del mismo. Otra es buscar la manera de automatizar lo más
posible toda la cadena de generación y despliegue del mismo.

Eso es lo que queremos decir cuando hablamos de Integración Continua y Despliegue
Continuo (CI/CD).

:::skip
Busca automatizar el flujo desde los últimos cambios realizados a la aplicación
hasta su despliegue en producción, asegurando que cumple con los requisitos básicos
de calidad, así como con el contexto necesario para su monitoreo continuo en día 2.
:::


```
+-------------------------+     +-------------------+
|           CI            |     |        CD         |
| Código -> Build -> Test | --> | Deploy -> Monitor |
|          Day 1          |     |       Day 2       |
+-------------------------+     +-------------------+
```
:::

