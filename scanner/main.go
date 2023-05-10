package main

import (
	"encoding/csv"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"strconv"
	"time"
  _ "github.com/joho/godotenv/autoload"
)

type ListWaypointResponse struct {
  Symbol string
  Type string
  SystemSymbol string
  X int
  Y int
  Traits []struct {
    Symbol string
    Name string
    Description string
  }
}

type ListSystemResponse struct {
  Symbol string
  SectorSymbol string
  Type string
  X int
  Y int
  Waypoints []struct {
    Symbol string
    Type string
    X int
    Y int
  }
  Factions []struct {
    Symbol string
  }
}

type GetMarketResponse struct {
  Exports []struct {
    Symbol string
    Name string
    Description string
  }
  Imports []struct {
    Symbol string
    Name string
    Description string
  }
  Exchange []struct {
    Symbol string
    Name string
    Description string
  }
}

type ResponseWithoutMeta[T any] struct {
  Data T
}

type Response[T any] struct {
  Data T
  Meta struct {
    Page int
    Total int
    Limit int
  }
}

func get[T any](url string, q url.Values) Response[T] {
  req, err := http.NewRequest("GET", url, nil)
  if err != nil {
    panic(err)
  }

  req.Header.Add("Authorization", fmt.Sprintf("Bearer %s", os.Getenv("API_KEY")))
  que := req.URL.Query()
  for key, values := range q {
    for _, value := range values {
      que.Add(key, value)
    }
  }

  req.URL.RawQuery = que.Encode()

  client := new(http.Client)

  res, err := client.Do(req)
  if err != nil {
    panic(err)
  }

  defer res.Body.Close()

  var value Response[T]

  body, _ := io.ReadAll(res.Body)
  if err := json.Unmarshal(body, &value); err != nil {
    panic(err)
  }

  time.Sleep(time.Millisecond * 500)

  return value
}

func main() {
  f, err := os.Create("systems.csv")
  if err != nil {
    panic(err)
  }
  r := flag.String("r", "FUEL", "resource")
  flag.Parse()
  for i := 0; i < 5; i++ {
    w := csv.NewWriter(f)
    q := url.Values{}
    q.Add("page", strconv.Itoa(i + 1))

    systems := get[[]ListSystemResponse]("https://api.spacetraders.io/v2/systems", q)

    for _, system := range systems.Data {
      points := get[[]ListWaypointResponse](fmt.Sprintf("https://api.spacetraders.io/v2/systems/%s/waypoints", system.Symbol), url.Values{})

      hasMarket := 0
      commonMetal := 0
      preciousMetal := 0
      rareMetal := 0
      mineral := 0
      iceCrystal := 0
      resource := 0
      for _, point := range points.Data {
        isMarket := false
        for _, trait := range point.Traits {
          switch trait.Symbol {
            case "MARKETPLACE", "BLACK_MARKET":
              hasMarket++
              isMarket = true
            case "COMMON_METAL_DEPOSITS":
              commonMetal++
            case "PRECIOUS_METAL_DEPOSITS":
              preciousMetal++
            case "RARE_METAL_DEPOSITS":
              rareMetal++
            case "MINERAL_DEPOSITS":
              mineral++
            case "ICE_CRYSTALS":
              iceCrystal++
          }
        }

        if isMarket {
          market := get[GetMarketResponse](fmt.Sprintf("https://api.spacetraders.io/v2/systems/%s/waypoints/%s/market", system.Symbol, point.Symbol), url.Values{})
          fmt.Printf("----------- %s -----------\n", point.Symbol)
          fmt.Println("IMPORTS")
          for _, good := range market.Data.Imports {
            fmt.Printf("\t%s\n", good.Symbol)
            if good.Symbol == *r {
              resource--
            }
          }
          fmt.Println("EXPORTS")
          for _, good := range market.Data.Exports {
            fmt.Printf("\t%s\n", good.Symbol)
            if good.Symbol == *r {
              resource++
            }
          }
          fmt.Println("EXCHANGES")
          for _, good := range market.Data.Exchange {
            fmt.Printf("\t%s\n", good.Symbol)
          }
          fmt.Println()
        }
      }
      data := []string{system.Symbol, strconv.Itoa(system.X), strconv.Itoa(system.Y), strconv.Itoa(hasMarket), strconv.Itoa(commonMetal), strconv.Itoa(preciousMetal), strconv.Itoa(rareMetal), strconv.Itoa(mineral), strconv.Itoa(iceCrystal), strconv.Itoa(resource)}
      err := w.Write(data)
      if err != nil {
        panic(err)
      }
    }
    w.Flush()
  }
}
