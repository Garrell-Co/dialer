"use client";

import { cn } from "../lib/utils";
import { signInWithEmailOtp, signInWithPassword } from "../auth/auth-actions";
import { Button } from "./ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "./ui/card";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useActionState, useEffect, useState } from "react";
import { useFormStatus } from "react-dom";
import { CheckCircle2, XCircle } from "lucide-react";

function SubmitButton() {
  const { pending } = useFormStatus();
  return (
    <Button type="submit" className="w-full" disabled={pending}>
      {pending ? "Logging in..." : "Login"}
    </Button>
  );
}

type SignInState =
  | { error: string }
  | { success: true; redirectTo: string }
  | null;

type EmailOtpState = { error?: string; success?: boolean } | null;

async function signInAction(
  prevState: SignInState,
  formData: FormData,
): Promise<SignInState> {
  const result = await signInWithPassword(formData);
  return result;
}

async function emailOtpAction(
  prevState: EmailOtpState,
  formData: FormData,
): Promise<EmailOtpState> {
  const result = await signInWithEmailOtp(formData);
  return result;
}

export function LoginForm({
  className,
  ...props
}: React.ComponentPropsWithoutRef<"div">) {
  const [state, formAction] = useActionState(signInAction, null);
  const [emailOtpState, emailOtpFormAction] = useActionState(
    emailOtpAction,
    null,
  );
  const [isModalOpen, setIsModalOpen] = useState(false);
  const router = useRouter();

  useEffect(() => {
    if (state && "success" in state && state.success && state.redirectTo) {
      router.push(state.redirectTo);
    }
  }, [state, router]);

  useEffect(() => {
    if (emailOtpState?.success || emailOtpState?.error) {
      const timer = setTimeout(() => {
        setIsModalOpen(false);
      }, 3000);
      return () => clearTimeout(timer);
    }
  }, [emailOtpState]);

  return (
    <div className={cn("flex flex-col gap-6", className)} {...props}>
      <Card>
        <CardHeader>
          <CardTitle className="text-2xl">Login</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-col gap-6">
            <Button
              type="button"
              variant="outline"
              className="w-full"
              onClick={() => setIsModalOpen(true)}
            >
              Login via email
            </Button>
            <div className="relative">
              <div className="absolute inset-0 flex items-center">
                <span className="w-full border-t" />
              </div>
              <div className="relative flex justify-center text-xs uppercase">
                <span className="bg-background px-2 text-muted-foreground">
                  Or
                </span>
              </div>
            </div>
            <form action={formAction}>
              <div className="flex flex-col gap-6">
                <div className="grid gap-2">
                  <Label htmlFor="password-email">Email</Label>
                  <Input
                    id="password-email"
                    name="email"
                    type="email"
                    placeholder="m@example.com"
                    required
                  />
                </div>
                <div className="grid gap-2">
                  <div className="flex items-center">
                    <Label htmlFor="password">Password</Label>
                    <Link
                      href="/auth/forgot-password"
                      className="ml-auto inline-block text-sm underline-offset-4 hover:underline"
                    >
                      Forgot your password?
                    </Link>
                  </div>
                  <Input
                    id="password"
                    name="password"
                    type="password"
                    required
                  />
                </div>
                {state && "error" in state && (
                  <p className="text-sm text-red-500">{state.error}</p>
                )}
                <SubmitButton />
              </div>
              <div className="mt-4 text-center text-sm">
                Don&apos;t have an account?{" "}
                <Link
                  href="/auth/sign-up"
                  className="underline underline-offset-4"
                >
                  Sign up
                </Link>
              </div>
            </form>
          </div>
        </CardContent>
      </Card>
      {isModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div
            className="fixed inset-0 bg-black/50"
            onClick={() => setIsModalOpen(false)}
          />
          <Card className="relative z-50 w-full max-w-md mx-4">
            <CardHeader>
              <CardTitle>Login via email</CardTitle>
              <CardDescription>
                Enter your email address and we&apos;ll send you a one-time
                login link.
              </CardDescription>
            </CardHeader>
            <CardContent>
              {emailOtpState?.success ? (
                <div className="flex flex-col items-center justify-center gap-4 py-4">
                  <CheckCircle2 className="h-16 w-16 text-green-600 animate-scale-in" />
                  <div className="text-center space-y-2 animate-fade-in [animation-delay:0.2s]">
                    <p className="text-lg font-semibold text-green-600">
                      Magic link sent!
                    </p>
                    <p className="text-sm text-muted-foreground">
                      Check your email for a login link. This window will close
                      automatically.
                    </p>
                  </div>
                </div>
              ) : emailOtpState?.error ? (
                <div className="flex flex-col items-center justify-center gap-4 py-4">
                  <XCircle className="h-16 w-16 text-red-600 animate-scale-in" />
                  <div className="text-center space-y-2 animate-fade-in [animation-delay:0.2s]">
                    <p className="text-lg font-semibold text-red-600">
                      Failed to send link
                    </p>
                    <p className="text-sm text-red-500">
                      {emailOtpState.error}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      This window will close automatically.
                    </p>
                  </div>
                </div>
              ) : (
                <form action={emailOtpFormAction} className="space-y-4">
                  <div className="grid gap-2">
                    <Label htmlFor="otp-email">Email</Label>
                    <Input
                      id="otp-email"
                      name="email"
                      type="email"
                      placeholder="m@example.com"
                      required
                    />
                  </div>
                  <div className="flex gap-2">
                    <Button
                      type="button"
                      variant="outline"
                      className="flex-1"
                      onClick={() => setIsModalOpen(false)}
                    >
                      Cancel
                    </Button>
                    <Button
                      type="submit"
                      className="flex-1"
                      disabled={emailOtpState?.success}
                    >
                      Send magic link
                    </Button>
                  </div>
                </form>
              )}
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
