import { createClient } from "@/lib/supabase/server";
import { type EmailOtpType } from "@supabase/supabase-js";
import { redirect } from "next/navigation";
import { type NextRequest } from "next/server";

export async function GET(request: NextRequest) {
  const { searchParams } = new URL(request.url);
  const token_hash = searchParams.get("token_hash");
  const type = searchParams.get("type") as EmailOtpType | null;
  const next = searchParams.get("next") ?? "/";

  console.log("[auth/confirm] Incoming request", {
    hasTokenHash: Boolean(token_hash),
    type,
    next,
  });

  // Normalise the "next" param so we always redirect to a path within this app.
  // The magic link currently sends an absolute URL (e.g. http://127.0.0.1:3000/protected),
  // but Next.js' redirect helper works best with relative paths.
  const getSafeRedirectPath = (value: string | null) => {
    if (!value) return "/";

    try {
      // If it's an absolute URL, strip it down to just path + search.
      const url = new URL(value);
      return url.pathname + url.search || "/";
    } catch {
      // If it's already a relative path and starts with "/", trust it.
      if (value.startsWith("/")) return value;
      // Fallback to root to avoid open redirects.
      return "/";
    }
  };

  if (token_hash && type) {
    const supabase = await createClient();

    console.log("[auth/confirm] Verifying OTP", {
      type,
      // Avoid logging the full token for security; just confirm presence
      hasTokenHash: true,
    });

    const { data, error } = await supabase.auth.verifyOtp({
      type,
      token_hash,
    });
    if (!error) {
      console.log("[auth/confirm] OTP verified successfully, redirecting", {
        hasSession: Boolean(data?.session),
        hasUser: Boolean(data?.user),
        redirectTo: getSafeRedirectPath(next),
      });
      // redirect user to specified redirect URL or root of app
      redirect(getSafeRedirectPath(next));
    }

    console.error("[auth/confirm] OTP verification failed", {
      type,
      error: error.message,
    });
    // redirect the user to an error page with some instructions
    redirect(
      `/auth/error?error=${encodeURIComponent(error?.message ?? "Unknown error")}`,
    );
  }

  // redirect the user to an error page with some instructions
  console.warn("[auth/confirm] Missing token_hash or type in request");
  redirect("/auth/error?error=No%20token%20hash%20or%20type");
}
